//! Track identification and metadata writing handlers.

use iced::Task;
use std::path::PathBuf;

use crate::{enrichment, metadata};

use super::super::messages::Message;
use super::super::state::LoadedState;
use super::load_tracks_task;

/// Handle enrichment-related messages
pub fn handle_enrichment(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::EnrichmentApiKeyChanged(key) => {
            s.enrichment.api_key = key;
        }
        Message::EnrichmentTrackSelected(idx) => {
            s.enrichment.selected_track = Some(idx);
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;
        }
        Message::EnrichmentIdentifyPressed => {
            let Some(idx) = s.enrichment.selected_track else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(idx) else {
                return Task::none();
            };

            if s.enrichment.api_key.is_empty() {
                s.enrichment.last_error =
                    Some("API key required. Get one at acoustid.org".to_string());
                return Task::none();
            }

            if !s.enrichment.fpcalc_available {
                s.enrichment.last_error =
                    Some("fpcalc not installed. Run 'check-tools' for help.".to_string());
                return Task::none();
            }

            s.enrichment.is_identifying = true;
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;

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
                Message::EnrichmentIdentifyResult,
            );
        }
        Message::EnrichmentIdentifyResult(result) => {
            s.enrichment.is_identifying = false;
            match result {
                Ok(identification) => {
                    s.enrichment.last_result = Some(identification);
                    s.enrichment.last_error = None;
                }
                Err(e) => {
                    s.enrichment.last_result = None;
                    s.enrichment.last_error = Some(e);
                }
            }
        }
        Message::EnrichmentClearResult => {
            s.enrichment.last_result = None;
            s.enrichment.last_error = None;
        }
        Message::EnrichmentWriteTagsPressed => {
            let Some(idx) = s.enrichment.selected_track else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(idx) else {
                return Task::none();
            };
            let Some(ref result) = s.enrichment.last_result else {
                return Task::none();
            };

            let path = PathBuf::from(&track.path);
            let identified = result.track.clone();

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
                Message::EnrichmentWriteTagsResult,
            );
        }
        Message::EnrichmentWriteTagsResult(result) => {
            match result {
                Ok(count) => {
                    s.status_message = format!("✓ Tags written ({} fields updated)", count);
                    // Reload tracks to show updated metadata
                    return load_tracks_task(s.pool.clone());
                }
                Err(e) => {
                    s.enrichment.last_error = Some(format!("Failed to write tags: {}", e));
                }
            }
        }
        _ => {}
    }
    Task::none()
}
