//! Track identification and metadata writing handlers.

use iced::Task;
use std::path::PathBuf;

use crate::{enrichment, metadata};

use super::super::messages::Message;
use super::super::state::{EnrichmentResult, LoadedState, ResultStatus};
use super::load_tracks_task;

/// Handle enrichment-related messages (single track - Settings pane)
pub fn handle_enrichment(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        Message::EnrichmentApiKeyChanged(key) => {
            s.enrichment.api_key = key.clone();
            // Also update the enrichment pane
            s.enrichment_pane.api_key = key;
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

/// Handle enrich pane messages (batch operations)
pub fn handle_enrich_pane(s: &mut LoadedState, msg: Message) -> Task<Message> {
    match msg {
        // Track selection
        Message::EnrichAddFromLibrary => {
            // Get currently selected/filtered tracks from library
            // For now, add all filtered tracks (or first 50 if too many)
            let indices_to_add: Vec<usize> = if s.filtered_indices.is_empty() {
                (0..s.tracks.len().min(50)).collect()
            } else {
                s.filtered_indices.iter().take(50).copied().collect()
            };

            for idx in indices_to_add {
                if !s.enrichment_pane.selected_tracks.contains(&idx) {
                    s.enrichment_pane.selected_tracks.push(idx);
                    // Auto-check new tracks
                    let pos = s.enrichment_pane.selected_tracks.len() - 1;
                    s.enrichment_pane.checked_tracks.insert(pos);
                }
            }
        }
        Message::EnrichAddTracks(indices) => {
            for idx in indices {
                if !s.enrichment_pane.selected_tracks.contains(&idx) {
                    s.enrichment_pane.selected_tracks.push(idx);
                    let pos = s.enrichment_pane.selected_tracks.len() - 1;
                    s.enrichment_pane.checked_tracks.insert(pos);
                }
            }
        }
        Message::EnrichRemoveTrack(pos) => {
            if pos < s.enrichment_pane.selected_tracks.len() {
                s.enrichment_pane.selected_tracks.remove(pos);
                s.enrichment_pane.checked_tracks.remove(&pos);
                // Re-index checked tracks above this position
                let mut new_checked = std::collections::HashSet::new();
                for &i in &s.enrichment_pane.checked_tracks {
                    if i > pos {
                        new_checked.insert(i - 1);
                    } else {
                        new_checked.insert(i);
                    }
                }
                s.enrichment_pane.checked_tracks = new_checked;
            }
        }
        Message::EnrichClearTracks => {
            s.enrichment_pane.selected_tracks.clear();
            s.enrichment_pane.checked_tracks.clear();
            s.enrichment_pane.results.clear();
        }
        Message::EnrichTrackChecked(pos, checked) => {
            if checked {
                s.enrichment_pane.checked_tracks.insert(pos);
            } else {
                s.enrichment_pane.checked_tracks.remove(&pos);
            }
        }

        // Options
        Message::EnrichFillOnlyToggled(fill_only) => {
            s.enrichment_pane.fill_only = fill_only;
        }
        Message::EnrichFetchCoverArtToggled(fetch) => {
            s.enrichment_pane.fetch_cover_art = fetch;
        }

        // Batch identification
        Message::EnrichBatchIdentify => {
            if s.enrichment_pane.api_key.is_empty() {
                s.status_message = "API key required".to_string();
                return Task::none();
            }
            if !s.enrichment_pane.fpcalc_available {
                s.status_message = "fpcalc not installed".to_string();
                return Task::none();
            }
            if s.enrichment_pane.checked_tracks.is_empty() {
                return Task::none();
            }

            s.enrichment_pane.is_identifying = true;
            s.enrichment_pane.results.clear();

            // Get paths for checked tracks
            let tracks_to_process: Vec<(usize, PathBuf)> = s
                .enrichment_pane
                .checked_tracks
                .iter()
                .filter_map(|&pos| {
                    s.enrichment_pane
                        .selected_tracks
                        .get(pos)
                        .and_then(|&track_idx| s.tracks.get(track_idx))
                        .map(|track| (pos, PathBuf::from(&track.path)))
                })
                .collect();

            if tracks_to_process.is_empty() {
                s.enrichment_pane.is_identifying = false;
                return Task::none();
            }

            // Start with first track
            let (first_pos, first_path) = tracks_to_process[0].clone();
            let api_key = s.enrichment_pane.api_key.clone();

            // Store remaining tracks for sequential processing
            // We'll process one at a time to respect rate limits
            return Task::perform(
                async move {
                    let config = enrichment::EnrichmentConfig {
                        acoustid_api_key: api_key,
                        min_confidence: 0.5,
                        use_musicbrainz: true,
                        ..Default::default()
                    };
                    let service = enrichment::EnrichmentService::new(config);
                    let result = service
                        .identify_track(&first_path)
                        .await
                        .map_err(|e| e.to_string());
                    (first_pos, result)
                },
                |(pos, result)| Message::EnrichBatchIdentifyResult(pos, result),
            );
        }

        Message::EnrichBatchIdentifyResult(pos, result) => {
            // Create result entry
            let enrich_result = match result {
                Ok(ref identification) => {
                    let mut changes = Vec::new();
                    if identification.track.title.is_some() {
                        changes.push("title".to_string());
                    }
                    if identification.track.artist.is_some() {
                        changes.push("artist".to_string());
                    }
                    if identification.track.album.is_some() {
                        changes.push("album".to_string());
                    }
                    if identification.track.year.is_some() {
                        changes.push("year".to_string());
                    }

                    let result_status = if identification.score >= 0.9 {
                        ResultStatus::Success
                    } else {
                        ResultStatus::Warning
                    };

                    EnrichmentResult {
                        track_index: pos,
                        status: result_status,
                        title: identification.track.title.clone(),
                        artist: identification.track.artist.clone(),
                        album: identification.track.album.clone(),
                        confidence: Some(identification.score),
                        changes,
                        error: None,
                        confirmed: identification.score >= 0.7, // Auto-confirm high confidence
                        identification: Some(identification.clone()),
                    }
                }
                Err(ref e) => EnrichmentResult {
                    track_index: pos,
                    status: ResultStatus::Error,
                    title: None,
                    artist: None,
                    album: None,
                    confidence: None,
                    changes: vec![],
                    error: Some(e.clone()),
                    confirmed: false,
                    identification: None,
                },
            };

            s.enrichment_pane.results.push(enrich_result);

            // Check if there are more tracks to process
            let processed_positions: std::collections::HashSet<usize> = s
                .enrichment_pane
                .results
                .iter()
                .map(|r| r.track_index)
                .collect();

            let next_track = s
                .enrichment_pane
                .checked_tracks
                .iter()
                .find(|&&p| !processed_positions.contains(&p))
                .copied();

            if let Some(next_pos) = next_track
                && let Some(&track_idx) = s.enrichment_pane.selected_tracks.get(next_pos)
                && let Some(track) = s.tracks.get(track_idx)
            {
                // Process next track
                let path = PathBuf::from(&track.path);
                let api_key = s.enrichment_pane.api_key.clone();

                return Task::perform(
                    async move {
                        // Small delay for rate limiting
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                        let config = enrichment::EnrichmentConfig {
                            acoustid_api_key: api_key,
                            min_confidence: 0.5,
                            use_musicbrainz: true,
                            ..Default::default()
                        };
                        let service = enrichment::EnrichmentService::new(config);
                        let result = service
                            .identify_track(&path)
                            .await
                            .map_err(|e| e.to_string());
                        (next_pos, result)
                    },
                    |(pos, result)| Message::EnrichBatchIdentifyResult(pos, result),
                );
            }

            // All done
            s.enrichment_pane.is_identifying = false;
            let success_count = s
                .enrichment_pane
                .results
                .iter()
                .filter(|r| r.status == ResultStatus::Success)
                .count();
            s.status_message = format!(
                "Identification complete: {} of {} matched",
                success_count,
                s.enrichment_pane.results.len()
            );
        }

        Message::EnrichBatchComplete => {
            s.enrichment_pane.is_identifying = false;
        }

        // Result actions
        Message::EnrichReviewResult(idx) => {
            // Toggle confirmed status for review
            if let Some(result) = s.enrichment_pane.results.get_mut(idx) {
                result.confirmed = !result.confirmed;
            }
        }

        Message::EnrichWriteResult(idx) => {
            let Some(result) = s.enrichment_pane.results.get(idx) else {
                return Task::none();
            };
            let Some(ref identification) = result.identification else {
                return Task::none();
            };
            let Some(&track_idx) = s.enrichment_pane.selected_tracks.get(result.track_index) else {
                return Task::none();
            };
            let Some(track) = s.tracks.get(track_idx) else {
                return Task::none();
            };

            let path = PathBuf::from(&track.path);
            let identified = identification.track.clone();
            let fill_only = s.enrichment_pane.fill_only;

            return Task::perform(
                async move {
                    let options = metadata::WriteOptions2 {
                        only_fill_empty: fill_only,
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

        Message::EnrichWriteAllConfirmed => {
            // Collect all confirmed results with identifications
            let to_write: Vec<(PathBuf, enrichment::domain::IdentifiedTrack)> = s
                .enrichment_pane
                .results
                .iter()
                .filter(|r| r.confirmed && r.identification.is_some())
                .filter_map(|r| {
                    let track_idx = s.enrichment_pane.selected_tracks.get(r.track_index)?;
                    let track = s.tracks.get(*track_idx)?;
                    let identification = r.identification.as_ref()?;
                    Some((PathBuf::from(&track.path), identification.track.clone()))
                })
                .collect();

            if to_write.is_empty() {
                s.status_message = "No confirmed results to write".to_string();
                return Task::none();
            }

            let fill_only = s.enrichment_pane.fill_only;
            let _count = to_write.len();

            return Task::perform(
                async move {
                    let mut success = 0;
                    let mut errors = Vec::new();

                    for (path, identified) in to_write {
                        let options = metadata::WriteOptions2 {
                            only_fill_empty: fill_only,
                            write_musicbrainz_ids: true,
                        };
                        match metadata::write(&path, &identified, &options) {
                            Ok(_) => success += 1,
                            Err(e) => errors.push(format!("{}: {}", path.display(), e)),
                        }
                    }

                    if errors.is_empty() {
                        Ok(success)
                    } else {
                        Err(format!(
                            "{} succeeded, {} failed: {}",
                            success,
                            errors.len(),
                            errors.join("; ")
                        ))
                    }
                },
                move |result: Result<usize, String>| match result {
                    Ok(n) => Message::EnrichmentWriteTagsResult(Ok(n)),
                    Err(e) => Message::EnrichmentWriteTagsResult(Err(e)),
                },
            );
        }

        Message::EnrichExportReport => {
            // Generate a simple text report
            let mut report = String::new();
            report.push_str("Music Minder Enrichment Report\n");
            report.push_str("==============================\n\n");

            for (i, result) in s.enrichment_pane.results.iter().enumerate() {
                let status = match result.status {
                    ResultStatus::Success => "✓",
                    ResultStatus::Warning => "⚠",
                    ResultStatus::Error => "✗",
                    ResultStatus::Pending => "…",
                };

                let title = result.title.as_deref().unwrap_or("Unknown");
                let confidence = result
                    .confidence
                    .map(|c| format!("{:.0}%", c * 100.0))
                    .unwrap_or_default();

                report.push_str(&format!(
                    "{} {}. {} ({})\n",
                    status,
                    i + 1,
                    title,
                    confidence
                ));

                if let Some(ref artist) = result.artist {
                    report.push_str(&format!("   Artist: {}\n", artist));
                }
                if let Some(ref album) = result.album {
                    report.push_str(&format!("   Album: {}\n", album));
                }
                if let Some(ref error) = result.error {
                    report.push_str(&format!("   Error: {}\n", error));
                }
                report.push('\n');
            }

            // For now, just copy to status message (could save to file later)
            s.status_message = format!(
                "Report: {} results ({} confirmed)",
                s.enrichment_pane.results.len(),
                s.enrichment_pane
                    .results
                    .iter()
                    .filter(|r| r.confirmed)
                    .count()
            );
            tracing::info!(target: "ui::enrich", "\n{}", report);
        }

        _ => {}
    }
    Task::none()
}
