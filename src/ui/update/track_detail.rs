//! Track detail modal handlers.
//!
//! Handles opening the track detail view, running identification,
//! and writing tags for a single track.

use iced::Task;
use std::path::PathBuf;

use crate::{enrichment, metadata};

use super::super::messages::Message;
use super::super::state::LoadedState;
use super::load_tracks_task;

/// Handle track detail messages
pub fn handle_track_detail(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::TrackDetailOpen(index) => {
            // Open detail view for this track
            let Some(track) = s.tracks.get(index) else {
                return Task::none();
            };

            s.track_detail.track_index = Some(index);
            s.track_detail.file_metadata = None;
            s.track_detail.full_metadata = None;
            s.track_detail.identification = None;
            s.track_detail.error = None;
            s.track_detail.is_identifying = false;
            s.track_detail.tags_written = false;

            // Read fresh metadata from the file (both simple and full)
            let path = PathBuf::from(&track.path);
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        let simple = metadata::read(&path).map_err(|e| e.to_string())?;
                        let full = metadata::read_full(&path).map_err(|e| e.to_string())?;
                        Ok::<_, String>((simple, full))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                },
                Message::TrackDetailRefreshed,
            );
        }

        Message::TrackDetailClose => {
            s.track_detail.track_index = None;
            s.track_detail.file_metadata = None;
            s.track_detail.full_metadata = None;
            s.track_detail.identification = None;
            s.track_detail.error = None;
            s.track_detail.is_identifying = false;
        }

        Message::TrackDetailRefreshed(result) => match result {
            Ok((simple, full)) => {
                s.track_detail.file_metadata = Some(simple);
                s.track_detail.full_metadata = Some(full);
            }
            Err(e) => {
                s.track_detail.error = Some(format!("Failed to read metadata: {}", e));
            }
        },

        Message::TrackDetailRefresh => {
            let Some(index) = s.track_detail.track_index else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(index) else {
                return Task::none();
            };

            let path = PathBuf::from(&track.path);
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        let simple = metadata::read(&path).map_err(|e| e.to_string())?;
                        let full = metadata::read_full(&path).map_err(|e| e.to_string())?;
                        Ok::<_, String>((simple, full))
                    })
                    .await
                    .map_err(|e| e.to_string())?
                },
                Message::TrackDetailRefreshed,
            );
        }

        Message::TrackDetailIdentify => {
            let Some(index) = s.track_detail.track_index else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(index) else {
                return Task::none();
            };

            if s.enrichment.api_key.is_empty() {
                s.track_detail.error =
                    Some("API key required. Configure in Settings → Enrichment".to_string());
                return Task::none();
            }

            if !s.enrichment.fpcalc_available {
                s.track_detail.error =
                    Some("fpcalc not installed. See Diagnostics for help.".to_string());
                return Task::none();
            }

            s.track_detail.is_identifying = true;
            s.track_detail.identification = None;
            s.track_detail.error = None;
            s.track_detail.tags_written = false;

            let path = PathBuf::from(&track.path);
            let api_key = s.enrichment.api_key.clone();

            return Task::perform(
                async move {
                    let config = enrichment::EnrichmentConfig {
                        acoustid_api_key: api_key,
                        min_confidence: 0.5,
                        use_musicbrainz: true,
                        ..Default::default()
                    };
                    let service = enrichment::EnrichmentService::new(config);
                    service
                        .identify_track(&path)
                        .await
                        .map_err(|e| e.to_string())
                },
                Message::TrackDetailIdentifyResult,
            );
        }

        Message::TrackDetailIdentifyResult(result) => {
            s.track_detail.is_identifying = false;
            match result {
                Ok(identification) => {
                    s.track_detail.identification = Some(identification);
                    s.track_detail.error = None;
                }
                Err(e) => {
                    s.track_detail.identification = None;
                    s.track_detail.error = Some(e);
                }
            }
        }

        Message::TrackDetailWriteTags => {
            let Some(index) = s.track_detail.track_index else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(index) else {
                return Task::none();
            };
            let Some(ref identification) = s.track_detail.identification else {
                return Task::none();
            };

            let path = PathBuf::from(&track.path);
            let identified = identification.track.clone();

            return Task::perform(
                async move {
                    let options = metadata::WriteOptions2 {
                        only_fill_empty: false, // Overwrite with enriched data
                        write_musicbrainz_ids: true,
                    };
                    tokio::task::spawn_blocking(move || {
                        metadata::write(&path, &identified, &options)
                            .map(|r| r.fields_updated)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    .map_err(|e| e.to_string())?
                },
                Message::TrackDetailWriteResult,
            );
        }

        Message::TrackDetailWriteResult(result) => {
            match result {
                Ok(count) => {
                    s.track_detail.tags_written = true;
                    s.status_message = format!("✓ Tags written ({} fields updated)", count);

                    // Refresh the file metadata to show updated values
                    if let Some(index) = s.track_detail.track_index
                        && let Some(track) = s.tracks.get(index)
                    {
                        let path = PathBuf::from(&track.path);
                        let refresh_task = Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    let simple =
                                        metadata::read(&path).map_err(|e| e.to_string())?;
                                    let full =
                                        metadata::read_full(&path).map_err(|e| e.to_string())?;
                                    Ok::<_, String>((simple, full))
                                })
                                .await
                                .map_err(|e| e.to_string())?
                            },
                            Message::TrackDetailRefreshed,
                        );

                        // Also reload tracks to update the library view
                        return Task::batch([refresh_task, load_tracks_task(s.pool.clone())]);
                    }
                }
                Err(e) => {
                    s.track_detail.error = Some(format!("Failed to write tags: {}", e));
                }
            }
        }

        _ => {}
    }
    Task::none()
}
