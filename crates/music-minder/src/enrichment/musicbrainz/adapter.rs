//! Adapter layer: Convert MusicBrainz DTOs to domain models
//!
//! This is the ONLY place where DTO types are converted to domain types.
//! This isolates API changes - if MusicBrainz changes their response format,
//! only this file and dto.rs need to change.

use super::dto;
use crate::enrichment::domain::{EnrichmentSource, IdentifiedTrack, TrackIdentification};

/// Release info extracted from MusicBrainz
struct ReleaseInfo {
    album: Option<String>,
    album_artist: Option<String>,
    release_id: Option<String>,
    release_group_id: Option<String>,
    track_number: Option<u32>,
    total_tracks: Option<u32>,
    disc_number: Option<u32>,
    total_discs: Option<u32>,
    year: Option<i32>,
}

/// Convert a MusicBrainz recording response to a TrackIdentification
pub fn to_identification(response: dto::RecordingResponse) -> TrackIdentification {
    // Build artist string from all credits
    let artist = build_artist_string(&response.artist_credit);
    let artist_id = response.artist_credit.first().map(|c| c.artist.id.clone());

    // Find the best release (prefer official albums)
    let release_info = extract_release_info(&response.releases);

    let (release_type, secondary_types) = extract_release_types(&response.releases);

    // Extract genres from tags, sorted by vote count
    let genres = extract_genres(&response.tags);

    let track = IdentifiedTrack {
        recording_id: Some(response.id),
        title: Some(response.title),
        artist,
        album_artist: release_info.album_artist,
        album: release_info.album,
        track_number: release_info.track_number,
        total_tracks: release_info.total_tracks,
        disc_number: release_info.disc_number,
        total_discs: release_info.total_discs,
        year: release_info.year,
        duration: response.length.map(std::time::Duration::from_millis),
        artist_id,
        release_id: release_info.release_id,
        release_group_id: release_info.release_group_id,
        release_type,
        secondary_types: secondary_types.unwrap_or_default(),
        genres,
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
fn extract_release_info(releases: &[dto::Release]) -> ReleaseInfo {
    // Prefer official album releases over singles/bootlegs
    let release = releases
        .iter()
        .find(|r| {
            r.status.as_deref() == Some("Official")
                && r.release_group
                    .as_ref()
                    .and_then(|rg| rg.primary_type.as_deref())
                    == Some("Album")
        })
        .or_else(|| {
            // Fall back to any official release
            releases
                .iter()
                .find(|r| r.status.as_deref() == Some("Official"))
        })
        .or_else(|| releases.first());

    let Some(release) = release else {
        return ReleaseInfo {
            album: None,
            album_artist: None,
            release_id: None,
            release_group_id: None,
            track_number: None,
            total_tracks: None,
            disc_number: None,
            total_discs: None,
            year: None,
        };
    };

    let album = Some(release.title.clone());
    let release_id = Some(release.id.clone());

    // Extract release group ID
    let release_group_id = release.release_group.as_ref().map(|rg| rg.id.clone());

    // Extract album artist from release artist credits
    let album_artist = release
        .artist_credit
        .as_ref()
        .and_then(|credits| build_artist_string(credits));

    // Total number of discs in the release
    let total_discs = if release.media.len() > 1 {
        Some(release.media.len() as u32)
    } else {
        None // Don't bother with disc number for single-disc releases
    };

    // Extract track position and disc number from media
    // We need to find which medium contains our track
    let (track_number, total_tracks, disc_number) = release
        .media
        .iter()
        .find_map(|m| {
            // Check if this medium has tracks (our track should be here)
            if let Some(track) = m.tracks.first() {
                let track_num = track.position;
                let disc_num = if release.media.len() > 1 {
                    m.position // Only include disc number for multi-disc releases
                } else {
                    None
                };
                Some((track_num, m.track_count, disc_num))
            } else {
                None
            }
        })
        .unwrap_or((None, None, None));

    // Parse year from date (YYYY, YYYY-MM, or YYYY-MM-DD)
    let year = release
        .date
        .as_ref()
        .and_then(|d| d.split('-').next())
        .and_then(|y| y.parse().ok());

    ReleaseInfo {
        album,
        album_artist,
        release_id,
        release_group_id,
        track_number,
        total_tracks,
        disc_number,
        total_discs,
        year,
    }
}

/// Extract genres from MusicBrainz tags, sorted by vote count (most popular first)
/// Takes the top 5 most-voted tags to avoid noise from low-confidence tags
fn extract_genres(tags: &[dto::Tag]) -> Vec<String> {
    let mut sorted_tags: Vec<_> = tags.iter().collect();
    sorted_tags.sort_by(|a, b| b.count.cmp(&a.count));

    sorted_tags
        .into_iter()
        .take(5) // Top 5 genres
        .filter(|t| t.count > 0) // Only include tags with positive votes
        .map(|t| {
            // Capitalize first letter of each word for display
            t.name
                .split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
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
            tags: vec![],
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

        assert_eq!(
            identification.track.recording_id,
            Some("rec-123".to_string())
        );
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
            artist_credit: None,
        }];

        let info = extract_release_info(&releases);

        assert_eq!(info.year, Some(1975));
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
                artist_credit: None,
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
                artist_credit: None,
            },
        ];

        let info = extract_release_info(&releases);

        assert_eq!(info.album, Some("Album".to_string()));
    }
}
