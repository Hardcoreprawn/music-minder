//! Audio playback and media controls handlers.
//!
//! # Control Flow
//!
//! All player commands flow through `handle_player()` which delegates to
//! internal helper functions. This ensures consistent behavior regardless
//! of entry point (UI button, keyboard shortcut, or OS media keys).
//!
//! # Event-Driven State Synchronization
//!
//! The audio thread runs asynchronously. Instead of polling `player.state()`,
//! we use an event-driven model:
//!
//! 1. UI sends commands (Play, Pause, etc.) to the audio thread
//! 2. Audio thread processes commands and emits `PlayerEvent`s
//! 3. UI receives events via `PlayerTick` and updates state
//!
//! This ensures the UI always reflects the *actual* state from the audio thread,
//! with no race conditions or stale data.
//!
//! # Debugging
//!
//! To trace the command → event flow, enable these log targets:
//!
//! ```powershell
//! # PowerShell
//! $env:RUST_LOG="player::events=debug,ui::commands=debug,ui::events=debug"
//! .\target\release\music-minder.exe
//!
//! # Or for all debug output:
//! $env:RUST_LOG="debug"; .\target\release\music-minder.exe
//! ```
//!
//! Log targets used in this module:
//! - `ui::commands` — Logs when `do_play()`, `do_pause()`, etc. are called
//! - `ui::events` — Logs when `PlayerEvent`s are received and processed
//!
//! See also: `player::events` in `src/player/audio.rs` for emission side.
//!
//! See `docs/ARCHITECTURE.md` for the full control flow diagram.

use iced::Task;
use std::path::PathBuf;

use crate::player::{self, Player, PlayerEvent};

use super::super::messages::Message;
use super::super::state::{CoverArtState, LoadedState};
use super::resolve_cover_art_task;

// ============================================================================
// Main message handler
// ============================================================================

/// Handle player-related messages.
///
/// All player commands (UI, keyboard, OS media keys) flow through here.
/// MediaControlCommand is converted to the equivalent action and uses
/// the same code path as direct UI messages.
pub fn handle_player(s: &mut LoadedState, msg: Message) -> Task<Message> {
    // Log entry to diagnose if handler is being reached
    let is_tick = matches!(msg, Message::PlayerTick);
    if !is_tick {
        tracing::debug!(target: "ui::handler", message = ?msg, "handle_player entered");
    }
    
    // Ensure player is initialized
    s.ensure_player();

    // Take the player out temporarily to avoid borrow conflicts
    let Some(mut player) = s.player.take() else {
        tracing::error!(target: "ui::handler", "Player is None! Cannot handle message");
        s.status_message = "Audio output not available".to_string();
        return Task::none();
    };

    let result = handle_player_inner(&mut player, s, msg);

    // Put the player back
    s.player = Some(player);

    result
}

/// Inner handler with player borrowed separately from state.
fn handle_player_inner(player: &mut Player, s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        // OS media control commands - dispatch to same handlers as UI
        Message::MediaControlCommand(cmd) => match cmd {
            player::MediaControlCommand::Play => do_play(player, s),
            player::MediaControlCommand::Pause => do_pause(player, s),
            player::MediaControlCommand::Toggle => do_toggle(player, s),
            player::MediaControlCommand::Stop => do_stop(player, s),
            player::MediaControlCommand::Next => do_next(player, s),
            player::MediaControlCommand::Previous => do_previous(player, s),
            player::MediaControlCommand::Seek(duration) => do_seek_absolute(player, s, duration),
            player::MediaControlCommand::SeekRelative(dir) => do_seek_relative(player, s, dir),
        },

        // UI messages - use same helpers
        Message::PlayerPlay => {
            // Simplified Play logic:
            // 1. If we have a track loaded (Paused), resume it.
            // 2. If we are Stopped but have a current track in queue, play it.
            // 3. If no current track but queue has items, start from top.
            // 4. Otherwise, warn.
            
            let status = s.player_state.status;
            let queue_has_current = player.queue().current().is_some();
            let queue_has_items = !player.queue().is_empty();
            
            tracing::debug!(target: "ui::commands", "PlayerPlay: status={:?}, queue_has_current={}, queue_has_items={}", status, queue_has_current, queue_has_items);

            if status == crate::player::PlaybackStatus::Paused {
                do_play(player, s);
            } else if queue_has_current {
                // Stopped but have a current track - reload and play
                if let Err(e) = player.play_current() {
                    s.status_message = format!("Play error: {}", e);
                } else {
                    on_track_changed(player, s);
                }
            } else if queue_has_items {
                // Start from the top of the queue
                if let Err(e) = player.skip_forward() {
                    s.status_message = format!("Play error: {}", e);
                } else {
                    on_track_changed(player, s);
                }
            } else {
                s.status_message = "Queue is empty. Add tracks or use Shuffle.".to_string();
            }
        }

        Message::PlayerPause => do_pause(player, s),
        Message::PlayerToggle => do_toggle(player, s),
        Message::PlayerStop => do_stop(player, s),
        Message::PlayerNext => do_next(player, s),
        Message::PlayerPrevious => do_previous(player, s),
        
        // Seek preview - just update UI display, no audio seek yet
        Message::PlayerSeekPreview(pos) => {
            tracing::trace!(
                target: "ui::seek",
                preview_pos = pos,
                current_pos_ms = s.player_state.position.as_millis(),
                duration_ms = s.player_state.duration.as_millis(),
                "Seek preview started/updated"
            );
            s.seek_preview = Some(pos);
        }
        
        // Seek release - perform actual seek using stored preview position, then clear preview
        Message::PlayerSeekRelease => {
            if let Some(pos) = s.seek_preview.take() {
                tracing::debug!(
                    target: "ui::seek",
                    seek_to = pos,
                    duration_ms = s.player_state.duration.as_millis(),
                    "Seek release - performing actual seek"
                );
                do_seek(player, s, pos);
            } else {
                tracing::warn!(target: "ui::seek", "Seek release called but no preview position set");
            }
        }

        Message::PlayerVolumeChanged(vol) => {
            tracing::debug!(
                target: "ui::volume",
                old_volume = s.player_state.volume,
                new_volume = vol,
                "Volume changed"
            );
            player.set_volume(vol);
            s.player_state.volume = vol;
        }

        Message::PlayerPlayTrack(idx) => {
            return play_track_at_index(player, s, idx);
        }

        Message::PlayerQueueTrack(idx) => {
            if let Some(track) = s.tracks.get(idx) {
                let path = PathBuf::from(&track.path);
                player.queue_file(path);
                s.status_message = format!("Queued: {}", track.title);
            }
        }

        Message::PlayerShuffleRandom => {
            shuffle_random_tracks(player, s);
        }

        Message::PlayerTick => {
            // === PHASE 1: Poll events from audio thread ===
            let events = player.poll_events();
            let event_count = events.len();
            
            // Log tick with full context for debugging timing issues
            tracing::debug!(
                target: "ui::tick",
                tick_events = event_count,
                ui_status = ?s.player_state.status,
                ui_pos_ms = s.player_state.position.as_millis(),
                ui_dur_ms = s.player_state.duration.as_millis(),
                seek_preview = ?s.seek_preview,
                "PlayerTick start"
            );
            
            for event in events {
                handle_player_event(event, player, s);
            }
            
            // === PHASE 2: Sync state from player (source of truth) ===
            // This happens AFTER events so we have the latest state
            let real_state = player.state();

            // Log any desyncs we detect and fix
            if s.player_state.status != real_state.status {
                tracing::warn!(
                    target: "ui::sync",
                    ui_status = ?s.player_state.status,
                    real_status = ?real_state.status,
                    "Fixed status desync"
                );
            }
            if s.player_state.duration != real_state.duration {
                tracing::warn!(
                    target: "ui::sync",
                    ui_dur_ms = s.player_state.duration.as_millis(),
                    real_dur_ms = real_state.duration.as_millis(),
                    "Fixed duration desync"
                );
            }
            // Log position drift if significant (>100ms) and not seeking
            if s.seek_preview.is_none() {
                let pos_diff = if s.player_state.position > real_state.position {
                    s.player_state.position - real_state.position
                } else {
                    real_state.position - s.player_state.position
                };
                if pos_diff > std::time::Duration::from_millis(100) {
                    tracing::debug!(
                        target: "ui::sync",
                        ui_pos_ms = s.player_state.position.as_millis(),
                        real_pos_ms = real_state.position.as_millis(),
                        diff_ms = pos_diff.as_millis(),
                        "Position drift detected"
                    );
                }
            }

            s.player_state = real_state;
            auto_queue_if_needed(player, s);
            
            // === PHASE 3: Update visualization if playing ===
            if s.player_state.status == crate::player::PlaybackStatus::Playing {
                if let Some(viz) = player.visualization() {
                    s.visualization = viz;
                }
            }
            
            // === PHASE 4: Poll media controls ===
            // IMPORTANT: Process commands directly here, NOT via handle_player()
            // to avoid re-entrancy issues (player is already borrowed)
            let commands: Vec<_> = s
                .media_controls
                .as_ref()
                .map(|mc| {
                    let mut cmds = Vec::new();
                    while let Some(cmd) = mc.try_recv_command() {
                        cmds.push(cmd);
                    }
                    cmds
                })
                .unwrap_or_default();

            // Process commands directly using the already-borrowed player
            for cmd in commands {
                tracing::debug!(target: "ui::media_control", command = ?cmd, "Processing media control");
                match cmd {
                    player::MediaControlCommand::Play => { do_play(player, s); }
                    player::MediaControlCommand::Pause => { do_pause(player, s); }
                    player::MediaControlCommand::Toggle => { do_toggle(player, s); }
                    player::MediaControlCommand::Stop => { do_stop(player, s); }
                    player::MediaControlCommand::Next => { do_next(player, s); }
                    player::MediaControlCommand::Previous => { do_previous(player, s); }
                    player::MediaControlCommand::Seek(duration) => { do_seek_absolute(player, s, duration); }
                    player::MediaControlCommand::SeekRelative(dir) => { do_seek_relative(player, s, dir); }
                }
            }
        }
        
        Message::PlayerEvent(event) => {
            handle_player_event(event, player, s);
        }

        Message::PlayerVisualizationTick => {
            // Now handled in PlayerTick, but kept for backwards compatibility
            if let Some(viz) = player.visualization() {
                s.visualization = viz;
            }
        }

        Message::PlayerVisualizationModeChanged(mode) => {
            s.visualization_mode = mode;
        }
        
        Message::MediaControlPoll => {
            // Now handled in PlayerTick, but kept as explicit fallback handler
            // (actual polling happens in PlayerTick to consolidate all event polling)
        }

        Message::PlayerSelectDevice(device_name) => {
            s.current_audio_device = device_name.clone();
            s.status_message = format!("Audio device: {} (restart to apply)", device_name);
        }

        _ => {}
    }
    Task::none()
}

// ============================================================================
// Event handler - processes confirmed state changes from audio thread
// ============================================================================

/// Handle a player event from the audio thread.
///
/// This is the ONLY place that updates player state in response to audio thread changes.
/// Events arrive in order, so rapid button mashing resolves deterministically.
fn handle_player_event(event: PlayerEvent, _player: &Player, s: &mut LoadedState) {
    match event {
        PlayerEvent::StatusChanged(status) => {
            tracing::debug!(target: "ui::events", "Received StatusChanged: {:?} -> {:?}", s.player_state.status, status);
            s.player_state.status = status;
            update_smtc_playback_state(s);
        }
        
        PlayerEvent::TrackLoaded { path, duration, sample_rate, channels, bits_per_sample, quality } => {
            tracing::debug!(target: "ui::events", "Received TrackLoaded: {:?}", path.file_name());
            s.player_state.current_track = Some(path);
            s.player_state.duration = duration;
            s.player_state.position = std::time::Duration::ZERO;
            s.player_state.sample_rate = sample_rate;
            s.player_state.channels = channels;
            s.player_state.bits_per_sample = bits_per_sample;
            s.player_state.quality = quality;
            
            // Sync metadata to OS media controls
            sync_metadata(s);
        }
        
        PlayerEvent::PositionChanged(position) => {
            s.player_state.position = position;
        }
        
        PlayerEvent::PlaybackFinished => {
            tracing::debug!(target: "ui::events", "Received PlaybackFinished");
            // Auto-queue next track if needed (handled in PlayerTick)
        }
        
        PlayerEvent::Error(err) => {
            tracing::warn!(target: "ui::events", "Received Error: {}", err);
            s.status_message = format!("Player error: {}", err);
        }
    }
}

// ============================================================================
// Internal helper functions - each action sends a command (no optimistic updates)
// ============================================================================

/// Play or resume playback.
///
/// Simply sends the Play command. State will be updated when we receive
/// the StatusChanged event from the audio thread.
fn do_play(player: &mut Player, s: &mut LoadedState) {
    tracing::debug!(target: "ui::commands", "do_play() called");
    if let Err(e) = player.play() {
        s.status_message = format!("Play error: {}", e);
    }
    // No state update here - wait for StatusChanged event
}

/// Pause playback.
fn do_pause(player: &mut Player, s: &mut LoadedState) {
    tracing::debug!(target: "ui::commands", "do_pause() called");
    if let Err(e) = player.pause() {
        s.status_message = format!("Pause error: {}", e);
    }
}

/// Toggle play/pause.
fn do_toggle(player: &mut Player, s: &mut LoadedState) {
    let ui_status = s.player_state.status;
    let real_status = player.state().status;
    tracing::debug!(
        target: "ui::commands",
        ui_status = ?ui_status,
        real_status = ?real_status,
        "do_toggle() called"
    );
    // Log if UI and real state differ (potential desync)
    if ui_status != real_status {
        tracing::warn!(
            target: "ui::commands",
            ui_status = ?ui_status,
            real_status = ?real_status,
            "Toggle called with status desync - using real state"
        );
    }
    if let Err(e) = player.toggle() {
        s.status_message = format!("Toggle error: {}", e);
        tracing::error!(target: "ui::commands", error = %e, "Toggle failed");
    }
}

/// Stop playback.
fn do_stop(player: &mut Player, s: &mut LoadedState) {
    tracing::debug!(target: "ui::commands", "do_stop() called");
    if let Err(e) = player.stop() {
        s.status_message = format!("Stop error: {}", e);
    }
}

/// Skip to next track.
fn do_next(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.skip_forward() {
        s.status_message = format!("Next error: {}", e);
    }
    on_track_changed(player, s);
}

/// Skip to previous track (or restart if >3s in).
fn do_previous(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.previous() {
        s.status_message = format!("Previous error: {}", e);
    }
    on_track_changed(player, s);
}

/// Seek to position (0.0 - 1.0).
fn do_seek(player: &mut Player, s: &mut LoadedState, position: f32) {
    if let Err(e) = player.seek(position) {
        s.status_message = format!("Seek error: {}", e);
    }
}

/// Seek to absolute duration.
fn do_seek_absolute(player: &mut Player, s: &mut LoadedState, duration: std::time::Duration) {
    let total = s.player_state.duration;
    if !total.is_zero() {
        let pos = duration.as_secs_f32() / total.as_secs_f32();
        do_seek(player, s, pos);
    }
}

/// Seek relative (forward/backward by 5 seconds).
fn do_seek_relative(player: &mut Player, s: &mut LoadedState, direction: souvlaki::SeekDirection) {
    let current_pos = s.player_state.position.as_secs_f32();
    let total = s.player_state.duration.as_secs_f32().max(1.0);
    let new_pos = match direction {
        souvlaki::SeekDirection::Forward => (current_pos + 5.0) / total,
        souvlaki::SeekDirection::Backward => (current_pos - 5.0).max(0.0) / total,
    };
    do_seek(player, s, new_pos.clamp(0.0, 1.0));
}

/// Called after skip operations to sync queue state for metadata.
/// The actual state update comes via TrackLoaded event.
fn on_track_changed(player: &Player, s: &mut LoadedState) {
    if let Some(current) = player.queue().current() {
        s.player_state.current_track = Some(current.path.clone());
    }
    sync_metadata(s);
}

// ============================================================================
// OS Media Controls (SMTC) helpers
// ============================================================================

/// Update SMTC playback state (playing/paused/stopped).
fn update_smtc_playback_state(s: &LoadedState) {
    if let Some(ref mc) = s.media_controls {
        let state = match s.player_state.status {
            player::PlaybackStatus::Playing => player::MediaPlaybackState::Playing,
            player::PlaybackStatus::Paused => player::MediaPlaybackState::Paused,
            player::PlaybackStatus::Stopped | player::PlaybackStatus::Loading => {
                player::MediaPlaybackState::Stopped
            }
        };
        mc.set_playback_state(state);
    }
}

/// Update OS media controls with current track metadata.
fn sync_metadata(s: &LoadedState) {
    if let Some(ref mc) = s.media_controls {
        if let Some(track_info) = s.current_track_info() {
            send_track_to_smtc(mc, track_info);
        } else if let Some(ref path) = s.player_state.current_track {
            tracing::warn!(
                "No track info found for path: {:?}, tracks loaded: {}",
                path,
                s.tracks.len()
            );
        }
    }
}

/// Send track metadata to SMTC.
fn send_track_to_smtc(mc: &player::MediaControlsHandle, track: &crate::db::TrackWithMetadata) {
    let duration = track
        .duration
        .map(|d| std::time::Duration::from_secs(d as u64))
        .unwrap_or_default();
    let meta = player::MediaControlsMetadata::with_title(&track.title)
        .artist(&track.artist_name)
        .album(&track.album_name)
        .duration(duration);
    tracing::info!(
        "Sending SMTC metadata: {} - {}",
        track.artist_name,
        track.title
    );
    mc.set_metadata(meta);
}

// ============================================================================
// Complex operations
// ============================================================================

/// Shuffle and play random tracks (clears current queue).
fn shuffle_random_tracks(player: &mut Player, s: &mut LoadedState) {
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();

    let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
    indices.shuffle(&mut rng);
    let count = 25.min(indices.len());

    player.queue_mut().clear();
    for &idx in indices.iter().take(count) {
        if let Some(track) = s.tracks.get(idx) {
            player.queue_file(PathBuf::from(&track.path));
        }
    }

    if let Err(e) = player.skip_forward() {
        s.status_message = format!("Shuffle error: {}", e);
    } else {
        s.status_message = format!("Shuffled {} random tracks", count);
        s.auto_queue_enabled = true;
        on_track_changed(player, s);
    }
}

/// Play a specific track by index and queue more from same artist.
fn play_track_at_index(player: &mut Player, s: &mut LoadedState, idx: usize) -> Task<Message> {
    let Some(track) = s.tracks.get(idx) else {
        return Task::none();
    };

    let path = PathBuf::from(&track.path);
    let artist = track.artist_name.clone();
    let title = track.title.clone();

    // Queue remaining tracks from the same artist
    let mut queued_count = 0;
    for (i, t) in s.tracks.iter().enumerate() {
        if i > idx && t.artist_name == artist && queued_count < 20 {
            player.queue_file(PathBuf::from(&t.path));
            queued_count += 1;
        }
    }

    if let Err(e) = player.play_file(path.clone()) {
        s.status_message = format!("Failed to play: {}", e);
        return Task::none();
    }

    s.status_message = format!("Playing: {} (+{} queued)", title, queued_count);
    s.auto_queue_enabled = true;

    // Use the same track-changed flow as everything else
    on_track_changed(player, s);

    // Trigger cover art resolution
    s.cover_art = CoverArtState {
        current: None,
        for_track: Some(path.clone()),
        loading: true,
        error: None,
    };
    resolve_cover_art_task(path, None)
}

/// Auto-queue more tracks when running low.
fn auto_queue_if_needed(player: &mut Player, s: &mut LoadedState) {
    if !s.auto_queue_enabled || s.tracks.is_empty() {
        return;
    }

    let remaining = player.queue().remaining_count();
    if remaining >= 5 {
        return;
    }

    use rand::seq::SliceRandom;
    let mut rng = rand::rng();

    let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
    indices.shuffle(&mut rng);
    let add_count = 8.min(indices.len());

    for &idx in indices.iter().take(add_count) {
        if let Some(track) = s.tracks.get(idx) {
            player.queue_file(PathBuf::from(&track.path));
        }
    }
    s.status_message = format!("Auto-queued {} more tracks", add_count);
}
