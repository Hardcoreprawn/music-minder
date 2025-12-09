//! Audio playback and media controls handlers.
//!
//! # Control Flow
//!
//! All player commands flow through `handle_player()` which delegates to
//! internal helper functions. This ensures consistent behavior regardless
//! of entry point (UI button, keyboard shortcut, or OS media keys).
//!
//! See `docs/ARCHITECTURE.md` for the full control flow diagram.

use iced::Task;
use std::path::PathBuf;

use crate::player::{self, Player};

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
    // Ensure player is initialized
    s.ensure_player();

    // Take the player out temporarily to avoid borrow conflicts
    let Some(mut player) = s.player.take() else {
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
        Message::MediaControlCommand(cmd) => {
            match cmd {
                player::MediaControlCommand::Play => do_play(player, s),
                player::MediaControlCommand::Pause => do_pause(player, s),
                player::MediaControlCommand::Toggle => do_toggle(player, s),
                player::MediaControlCommand::Stop => do_stop(player, s),
                player::MediaControlCommand::Next => do_next(player, s),
                player::MediaControlCommand::Previous => do_previous(player, s),
                player::MediaControlCommand::Seek(duration) => do_seek_absolute(player, s, duration),
                player::MediaControlCommand::SeekRelative(dir) => do_seek_relative(player, s, dir),
            }
        }

        // UI messages - use same helpers
        Message::PlayerPlay => {
            // If queue is empty and nothing playing, start random shuffle
            if player.queue_mut().is_empty() && s.player_state.current_track.is_none() {
                start_random_shuffle(player, s);
            } else {
                do_play(player, s);
            }
        }

        Message::PlayerPause => do_pause(player, s),
        Message::PlayerToggle => do_toggle(player, s),
        Message::PlayerStop => do_stop(player, s),
        Message::PlayerNext => do_next(player, s),
        Message::PlayerPrevious => do_previous(player, s),
        Message::PlayerSeek(pos) => do_seek(player, s, pos),

        Message::PlayerVolumeChanged(vol) => {
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
            s.player_state = player.state();
            auto_queue_if_needed(player, s);
        }

        Message::PlayerVisualizationTick => {
            if let Some(viz) = player.visualization() {
                s.visualization = viz;
            }
        }

        Message::PlayerVisualizationModeChanged(mode) => {
            s.visualization_mode = mode;
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
// Internal helper functions - each action implemented once
// ============================================================================

/// Play or resume playback.
fn do_play(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.play() {
        s.status_message = format!("Play error: {}", e);
    }
    sync_state(player, s);
}

/// Pause playback.
fn do_pause(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.pause() {
        s.status_message = format!("Pause error: {}", e);
    }
    sync_state(player, s);
}

/// Toggle play/pause.
fn do_toggle(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.toggle() {
        s.status_message = format!("Toggle error: {}", e);
    }
    sync_state(player, s);
}

/// Stop playback.
fn do_stop(player: &mut Player, s: &mut LoadedState) {
    if let Err(e) = player.stop() {
        s.status_message = format!("Stop error: {}", e);
    }
    sync_state(player, s);
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

// ============================================================================
// State synchronization helpers
// ============================================================================

/// Sync player state and update OS media controls playback status.
fn sync_state(player: &Player, s: &mut LoadedState) {
    s.player_state = player.state();
    update_smtc_playback_state(s);
}

/// Sync player state AND current track from the queue (single source of truth).
/// Use this after skip_forward/previous where the queue knows the new track immediately.
fn sync_state_with_queue(player: &Player, s: &mut LoadedState) {
    s.player_state = player.state();
    // Queue is the source of truth for current track - update player_state to match
    if let Some(current) = player.queue().current() {
        s.player_state.current_track = Some(current.path.clone());
    }
    update_smtc_playback_state(s);
}

/// Called after any operation that changes the current track.
/// This is the SINGLE place that syncs state + metadata after track changes.
fn on_track_changed(player: &Player, s: &mut LoadedState) {
    sync_state_with_queue(player, s);
    sync_metadata(s);
}

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

/// Start playback with random shuffled tracks.
fn start_random_shuffle(player: &mut Player, s: &mut LoadedState) {
    use rand::seq::SliceRandom;
    let mut rng = rand::rng();

    let mut indices: Vec<usize> = (0..s.tracks.len()).collect();
    indices.shuffle(&mut rng);
    let count = 25.min(indices.len());

    for &idx in indices.iter().take(count) {
        if let Some(track) = s.tracks.get(idx) {
            player.queue_file(PathBuf::from(&track.path));
        }
    }

    if let Err(e) = player.skip_forward() {
        s.status_message = format!("Play error: {}", e);
        s.player_state = player.state();
    } else {
        s.status_message = format!("Started shuffle with {} random tracks", count);
        s.auto_queue_enabled = true;
        on_track_changed(player, s);
    }
}

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
