//! Adapter layer: Convert MusicBrainz DTOs to domain models
//!
//! This is the ONLY place where DTO types are converted to domain types.
//! This isolates API changes - if MusicBrainz changes their response format,
//! only this file and dto.rs need to change.

use super::dto;
use crate::enrichment::domain::{EnrichmentSource, IdentifiedTrack, TrackIdentification};

/// Convert a MusicBrainz recording response to a TrackIdentification
pub fn to_identification(response: dto::RecordingResponse) -> TrackIdentification {
    // Build artist string from all credits
    let artist = build_artist_string(&response.artist_credit);
    let artist_id = response.artist_credit.first().map(|c| c.artist.id.clone());

    // Find the best release (prefer official albums)
    let (album, release_id, track_number, total_tracks, year) =
        extract_release_info(&response.releases);

    let (release_type, secondary_types) = extract_release_types(&response.releases);

    let track = IdentifiedTrack {
        recording_id: Some(response.id),
        title: Some(response.title),
        artist,
        album,
        track_number,
        total_tracks,
        year,
        duration: response
            .length
            .map(|ms| std::time::Duration::from_millis(ms)),
        artist_id,
        release_id,
        release_type,
        secondary_types: secondary_types.unwrap_or_default(),
    };

    TrackIdentification {
        score: 1.0, // MusicBrainz lookups by ID are exact matches
        track,
        source: EnrichmentSource::MusicBrainz,
    }
}

/// Build a combined artist string from artist credits
fn build_artist_string(credits: &[dto::ArtistCredit]) -> Option<String> {
    if credits.is_empty() {
        return None;
    }

    let mut result = String::new();
    for credit in credits {
        // Use credited name if available, otherwise official name
        let name = credit.name.as_ref().unwrap_or(&credit.artist.name);
        result.push_str(name);

        // Add join phrase if present (e.g., " & ", " feat. ")
        if let Some(ref join) = credit.joinphrase {
            result.push_str(join);
        }
    }

    Some(result)
}

/// Extract release type and secondary types from releases
fn extract_release_types(releases: &[dto::Release]) -> (Option<String>, Option<Vec<String>>) {
    let release = match releases.first() {
        Some(r) => r,
        None => return (None, None),
    };

    let release_type = release
        .release_group
        .as_ref()
        .and_then(|rg| rg.primary_type.clone());

    let secondary_types = None; // MusicBrainz DTO doesn't currently include secondary types

    (release_type, secondary_types)
}

/// Extract the best release info from available releases
fn extract_release_info(
    releases: &[dto::Release],
) -> (
    Option<String>,
    Option<String>,
    Option<u32>,
    Option<u32>,
    Option<i32>,
) {
    // Prefer official album releases over singles/bootlegs
    let release = releases
        .iter()
        .filter(|r| r.status.as_deref() == Some("Official"))
        .filter(|r| {
            r.release_group
                .as_ref()
                .and_then(|rg| rg.primary_type.as_deref())
                == Some("Album")
        })
        .next()
        .or_else(|| {
            // Fall back to any official release
            releases
                .iter()
                .filter(|r| r.status.as_deref() == Some("Official"))
                .next()
        })
        .or_else(|| releases.first());

    let Some(release) = release else {
        return (None, None, None, None, None);
    };

    let album = Some(release.title.clone());
    let release_id = Some(release.id.clone());

    // Extract track position from first medium
    let (track_number, total_tracks) = release
        .media
        .first()
        .map(|m| {
            let track_num = m.tracks.first().and_then(|t| t.position);
            (track_num, m.track_count)
        })
        .unwrap_or((None, None));

    // Parse year from date (YYYY, YYYY-MM, or YYYY-MM-DD)
    let year = release
        .date
        .as_ref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse().ok());

    (album, release_id, track_number, total_tracks, year)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_recording(id: &str, title: &str) -> dto::RecordingResponse {
        dto::RecordingResponse {
            id: id.to_string(),
            title: title.to_string(),
            length: None,
            disambiguation: None,
            artist_credit: vec![],
            releases: vec![],
        }
    }

    fn make_artist_credit(name: &str, join: Option<&str>) -> dto::ArtistCredit {
        dto::ArtistCredit {
            artist: dto::Artist {
                id: format!("{}-id", name.to_lowercase()),
                name: name.to_string(),
                sort_name: None,
                artist_type: None,
            },
            name: Some(name.to_string()),
            joinphrase: join.map(String::from),
        }
    }

    #[test]
    fn test_convert_minimal_recording() {
        let recording = make_recording("rec-123", "Test Song");

        let identification = to_identification(recording);

        assert_eq!(identification.track.recording_id, Some("rec-123".to_string()));
        assert_eq!(identification.track.title, Some("Test Song".to_string()));
        assert_eq!(identification.source, EnrichmentSource::MusicBrainz);
        assert_eq!(identification.score, 1.0);
    }

    #[test]
    fn test_build_single_artist() {
        let credits = vec![make_artist_credit("Queen", None)];

        let artist = build_artist_string(&credits);

        assert_eq!(artist, Some("Queen".to_string()));
    }

    #[test]
    fn test_build_collaboration_artist() {
        let credits = vec![
            make_artist_credit("Queen", Some(" & ")),
            make_artist_credit("David Bowie", None),
        ];

        let artist = build_artist_string(&credits);

        assert_eq!(artist, Some("Queen & David Bowie".to_string()));
    }

    #[test]
    fn test_extract_year_from_date() {
        let releases = vec![dto::Release {
            id: "rel-123".to_string(),
            title: "Test Album".to_string(),
            status: Some("Official".to_string()),
            date: Some("1975-10-31".to_string()),
            country: None,
            release_group: None,
            media: vec![],
        }];

        let (_, _, _, _, year) = extract_release_info(&releases);

        assert_eq!(year, Some(1975));
    }

    #[test]
    fn test_prefer_official_album() {
        let releases = vec![
            dto::Release {
                id: "single".to_string(),
                title: "Single".to_string(),
                status: Some("Official".to_string()),
                date: None,
                country: None,
                release_group: Some(dto::ReleaseGroup {
                    id: "rg-single".to_string(),
                    title: "Single".to_string(),
                    primary_type: Some("Single".to_string()),
                    first_release_date: None,
                }),
                media: vec![],
            },
            dto::Release {
                id: "album".to_string(),
                title: "Album".to_string(),
                status: Some("Official".to_string()),
                date: None,
                country: None,
                release_group: Some(dto::ReleaseGroup {
                    id: "rg-album".to_string(),
                    title: "Album".to_string(),
                    primary_type: Some("Album".to_string()),
                    first_release_date: None,
                }),
                media: vec![],
            },
        ];

        let (album, _, _, _, _) = extract_release_info(&releases);

        assert_eq!(album, Some("Album".to_string()));
    }
}
