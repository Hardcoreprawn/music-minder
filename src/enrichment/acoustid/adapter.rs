//! Adapter layer: Convert AcoustID DTOs to domain models
//!
//! This is the ONLY place where DTO types are converted to domain types.
//! This isolates API changes - if AcoustID changes their response format,
//! only this file and dto.rs need to change.

use super::dto;
use crate::enrichment::domain::{
    EnrichmentError, EnrichmentSource, IdentifiedTrack, TrackIdentification,
};

/// Convert an AcoustID lookup response to domain identifications
pub fn to_identifications(
    response: dto::LookupResponse,
) -> Result<Vec<TrackIdentification>, EnrichmentError> {
    if response.status != "ok" {
        let error = response.error.unwrap_or(dto::ApiError {
            code: -1,
            message: "Unknown error".to_string(),
        });
        return Err(EnrichmentError::ApiError(error.message));
    }

    Ok(response
        .results
        .into_iter()
        .flat_map(convert_result_to_identifications)
        .collect())
}

/// Convert a single AcoustID result to multiple TrackIdentifications
/// Each recording is expanded with all its release groups to enable better matching
fn convert_result_to_identifications(result: dto::LookupResult) -> Vec<TrackIdentification> {
    let score = result.score;
    
    result.recordings
        .into_iter()
        .flat_map(|recording| {
            convert_recording_to_identifications(recording, score)
        })
        .collect()
}

/// Convert a single recording to multiple identifications (one per release group)
fn convert_recording_to_identifications(
    recording: dto::Recording,
    acoustid_score: f32,
) -> Vec<TrackIdentification> {
    // Get artist info from first artist
    let (artist_name, artist_id) = recording
        .artists.first()
        .map(|a| (Some(a.name.clone()), Some(a.id.clone())))
        .unwrap_or((None, None));

    let title = recording.title.clone();
    let recording_id = recording.id.clone();
    let duration = recording.duration.map(|d| std::time::Duration::from_secs(d as u64));

    // If we have release groups, create one identification per release group
    if !recording.releasegroups.is_empty() {
        recording.releasegroups
            .into_iter()
            .map(|rg| {
                let track = IdentifiedTrack {
                    recording_id: Some(recording_id.clone()),
                    title: title.clone(),
                    artist: artist_name.clone(),
                    album: rg.title.clone(),
                    track_number: None,
                    total_tracks: None,
                    year: None,
                    duration,
                    artist_id: artist_id.clone(),
                    release_id: Some(rg.id),
                    release_type: rg.release_type,
                    secondary_types: rg.secondarytypes,
                };

                TrackIdentification {
                    score: acoustid_score,
                    track,
                    source: EnrichmentSource::AcoustId,
                }
            })
            .collect()
    } else {
        // Fall back to single identification without album info
        vec![TrackIdentification {
            score: acoustid_score,
            track: IdentifiedTrack {
                recording_id: Some(recording_id),
                title,
                artist: artist_name,
                album: None,
                track_number: None,
                total_tracks: None,
                year: None,
                duration,
                artist_id,
                release_id: None,
                release_type: None,
                secondary_types: vec![],
            },
            source: EnrichmentSource::AcoustId,
        }]
    }
}

/// Select the best identification from a list (highest score)
pub fn best_identification(identifications: Vec<TrackIdentification>) -> Option<TrackIdentification> {
    identifications.into_iter().max_by(|a, b| {
        a.score
            .partial_cmp(&b.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(status: &str, results: Vec<dto::LookupResult>) -> dto::LookupResponse {
        dto::LookupResponse {
            status: status.to_string(),
            results,
            error: None,
        }
    }

    fn make_result(id: &str, score: f32, recordings: Vec<dto::Recording>) -> dto::LookupResult {
        dto::LookupResult {
            id: id.to_string(),
            score,
            recordings,
        }
    }

    fn make_recording(id: &str, title: Option<&str>) -> dto::Recording {
        dto::Recording {
            id: id.to_string(),
            title: title.map(String::from),
            duration: None,
            artists: vec![],
            releases: vec![],
            releasegroups: vec![],
        }
    }

    #[test]
    fn test_convert_successful_response() {
        let response = make_response(
            "ok",
            vec![make_result(
                "aid-1",
                0.9,
                vec![make_recording("mbid-1", Some("Test Song"))],
            )],
        );

        let identifications = to_identifications(response).unwrap();
        assert_eq!(identifications.len(), 1);
        assert_eq!(identifications[0].track.title, Some("Test Song".to_string()));
        assert_eq!(identifications[0].score, 0.9);
        assert_eq!(identifications[0].source, EnrichmentSource::AcoustId);
    }

    #[test]
    fn test_convert_error_response() {
        let response = dto::LookupResponse {
            status: "error".to_string(),
            results: vec![],
            error: Some(dto::ApiError {
                code: 4,
                message: "rate limit".to_string(),
            }),
        };

        let result = to_identifications(response);
        assert!(matches!(result, Err(EnrichmentError::ApiError(_))));
    }

    #[test]
    fn test_skip_empty_recordings() {
        let response = make_response(
            "ok",
            vec![
                make_result("aid-1", 0.9, vec![]), // No recordings
                make_result("aid-2", 0.8, vec![make_recording("mbid-2", Some("Song"))]),
            ],
        );

        let identifications = to_identifications(response).unwrap();
        assert_eq!(identifications.len(), 1); // Only one valid result
    }

    #[test]
    fn test_best_identification_picks_highest_score() {
        let identifications = vec![
            TrackIdentification {
                score: 0.5,
                track: IdentifiedTrack {
                    title: Some("Low".to_string()),
                    ..Default::default()
                },
                source: EnrichmentSource::AcoustId,
            },
            TrackIdentification {
                score: 0.9,
                track: IdentifiedTrack {
                    title: Some("High".to_string()),
                    ..Default::default()
                },
                source: EnrichmentSource::AcoustId,
            },
        ];

        let best = best_identification(identifications).unwrap();
        assert_eq!(best.track.title, Some("High".to_string()));
        assert_eq!(best.score, 0.9);
    }
}
