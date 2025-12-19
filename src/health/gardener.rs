//! Background quality gardener for the music library.
//!
//! This worker runs in the background, gradually assessing track metadata quality
//! and flagging files that could benefit from enrichment. It's designed to be
//! non-intrusive - working during idle moments and never interfering with playback.
//!
//! # Design Philosophy
//!
//! The gardener "tends" to your library like a garden:
//! - Works slowly and steadily, not all at once
//! - Respects rate limits and system resources
//! - Prioritizes newly added or changed files
//! - Builds up quality data over time
//!
//! # Usage
//!
//! ```ignore
//! let gardener = QualityGardener::new(pool.clone());
//! gardener.start().await;
//! ```

use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::db::{
    QualityStats, TrackWithMetadata, get_tracks_needing_quality_check, update_track_quality,
};
use crate::health::{TrackQuality, assess_quality};

/// Configuration for the quality gardener.
#[derive(Debug, Clone)]
pub struct GardenerConfig {
    /// How often to check for work (default: 30 seconds)
    pub check_interval: Duration,
    /// How many tracks to process per batch (default: 10)
    pub batch_size: u32,
    /// Delay between processing individual tracks (default: 100ms)
    pub track_delay: Duration,
    /// Whether to run fingerprinting (requires fpcalc, uses network)
    pub enable_fingerprinting: bool,
}

impl Default for GardenerConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            batch_size: 10,
            track_delay: Duration::from_millis(100),
            enable_fingerprinting: false, // Start conservative
        }
    }
}

/// Commands that can be sent to the gardener.
#[derive(Debug)]
pub enum GardenerCommand {
    /// Process a specific track immediately
    ProcessTrack(i64),
    /// Process a batch of tracks
    ProcessBatch(Vec<i64>),
    /// Pause processing
    Pause,
    /// Resume processing
    Resume,
    /// Stop the gardener
    Stop,
}

/// Events emitted by the gardener.
#[derive(Debug, Clone)]
pub enum GardenerEvent {
    /// A track's quality was assessed
    TrackAssessed {
        track_id: i64,
        quality: TrackQuality,
    },
    /// A batch was completed
    BatchComplete { processed: usize, remaining: usize },
    /// Statistics updated
    StatsUpdated(QualityStats),
    /// Gardener paused
    Paused,
    /// Gardener resumed
    Resumed,
    /// Gardener stopped
    Stopped,
}

/// The quality gardener - tends to your music library in the background.
pub struct QualityGardener {
    pool: SqlitePool,
    config: GardenerConfig,
    command_tx: mpsc::Sender<GardenerCommand>,
    command_rx: Option<mpsc::Receiver<GardenerCommand>>,
    event_tx: Option<mpsc::Sender<GardenerEvent>>,
}

impl QualityGardener {
    /// Create a new gardener with default configuration.
    pub fn new(pool: SqlitePool) -> Self {
        Self::with_config(pool, GardenerConfig::default())
    }

    /// Create a new gardener with custom configuration.
    pub fn with_config(pool: SqlitePool, config: GardenerConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel(32);
        Self {
            pool,
            config,
            command_tx,
            command_rx: Some(command_rx),
            event_tx: None,
        }
    }

    /// Get a sender for commands.
    pub fn command_sender(&self) -> mpsc::Sender<GardenerCommand> {
        self.command_tx.clone()
    }

    /// Set the event sender for receiving updates.
    pub fn set_event_sender(&mut self, tx: mpsc::Sender<GardenerEvent>) {
        self.event_tx = Some(tx);
    }

    /// Start the gardener background task.
    ///
    /// Returns immediately - the gardener runs in a spawned task.
    pub fn start(mut self) -> tokio::task::JoinHandle<()> {
        let command_rx = self.command_rx.take().expect("Gardener already started");

        tokio::spawn(async move {
            self.run(command_rx).await;
        })
    }

    /// Main run loop.
    async fn run(&self, mut command_rx: mpsc::Receiver<GardenerCommand>) {
        let mut check_timer = interval(self.config.check_interval);
        let mut paused = false;

        tracing::info!(target: "gardener", "Quality gardener started");

        loop {
            tokio::select! {
                // Handle commands
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        GardenerCommand::ProcessTrack(id) => {
                            self.process_track_by_id(id).await;
                        }
                        GardenerCommand::ProcessBatch(ids) => {
                            for id in ids {
                                self.process_track_by_id(id).await;
                                tokio::time::sleep(self.config.track_delay).await;
                            }
                        }
                        GardenerCommand::Pause => {
                            paused = true;
                            self.emit(GardenerEvent::Paused).await;
                            tracing::debug!(target: "gardener", "Paused");
                        }
                        GardenerCommand::Resume => {
                            paused = false;
                            self.emit(GardenerEvent::Resumed).await;
                            tracing::debug!(target: "gardener", "Resumed");
                        }
                        GardenerCommand::Stop => {
                            self.emit(GardenerEvent::Stopped).await;
                            tracing::info!(target: "gardener", "Stopped");
                            break;
                        }
                    }
                }

                // Periodic check for work
                _ = check_timer.tick() => {
                    if !paused {
                        self.process_batch().await;
                    }
                }
            }
        }
    }

    /// Process a batch of unchecked tracks.
    async fn process_batch(&self) {
        let tracks =
            match get_tracks_needing_quality_check(&self.pool, self.config.batch_size).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(target: "gardener", "Failed to get tracks: {}", e);
                    return;
                }
            };

        if tracks.is_empty() {
            return;
        }

        tracing::debug!(target: "gardener", "Processing {} tracks", tracks.len());

        for track in &tracks {
            self.assess_track(track).await;
            tokio::time::sleep(self.config.track_delay).await;
        }

        // Get remaining count for event
        let remaining = get_tracks_needing_quality_check(&self.pool, 1)
            .await
            .map(|t| if t.is_empty() { 0 } else { 1 })
            .unwrap_or(0);

        self.emit(GardenerEvent::BatchComplete {
            processed: tracks.len(),
            remaining,
        })
        .await;
    }

    /// Process a specific track by ID.
    async fn process_track_by_id(&self, track_id: i64) {
        // Fetch the track with metadata
        let tracks: Vec<TrackWithMetadata> = match sqlx::query_as(
            r#"
            SELECT 
                t.id, t.title, t.path, t.duration, t.track_number,
                COALESCE(a.name, 'Unknown Artist') as artist_name,
                COALESCE(al.title, 'Unknown Album') as album_name,
                al.year,
                t.quality_score, t.quality_flags
            FROM tracks t
            LEFT JOIN artists a ON t.artist_id = a.id
            LEFT JOIN albums al ON t.album_id = al.id
            WHERE t.id = ?
            "#,
        )
        .bind(track_id)
        .fetch_all(&self.pool)
        .await
        {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!(target: "gardener", "Failed to fetch track {}: {}", track_id, e);
                return;
            }
        };

        if let Some(track) = tracks.first() {
            self.assess_track(track).await;
        }
    }

    /// Assess a single track's quality.
    async fn assess_track(&self, track: &TrackWithMetadata) {
        // Extract filename without extension for comparison
        let filename = std::path::Path::new(&track.path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Convert artist/album to Option<&str>, filtering out "Unknown" values
        let artist = if track.artist_name == "Unknown Artist" {
            None
        } else {
            Some(track.artist_name.as_str())
        };
        let album = if track.album_name == "Unknown Album" {
            None
        } else {
            Some(track.album_name.as_str())
        };

        // Start with basic metadata assessment
        let mut quality = assess_quality(
            &track.title,
            artist,
            album,
            track.year,
            track.track_number,
            filename,
            None, // Will be filled by verification if enabled
            None, // Will be filled by verification if enabled
        );

        // If fingerprinting is enabled, verify against AcoustID
        if self.config.enable_fingerprinting
            && let Some(verification) = self.verify_track(track).await
        {
            // Update quality based on verification result
            quality = self.apply_verification_to_quality(quality, &verification);
        }

        // Update database
        if let Err(e) = update_track_quality(&self.pool, track.id, &quality).await {
            tracing::warn!(target: "gardener", "Failed to update quality for {}: {}", track.id, e);
            return;
        }

        tracing::trace!(
            target: "gardener",
            "Assessed track {}: score={}, flags={:?}",
            track.id,
            quality.score,
            quality.flags
        );

        self.emit(GardenerEvent::TrackAssessed {
            track_id: track.id,
            quality,
        })
        .await;
    }

    /// Verify a track against fingerprint database.
    /// Returns None if verification couldn't be performed.
    async fn verify_track(
        &self,
        track: &TrackWithMetadata,
    ) -> Option<crate::health::VerificationResult> {
        use crate::enrichment::{acoustid, fingerprint};
        use crate::health::{
            ExistingMetadata, FingerprintMatch, ReleaseInfo, ReleaseType, VerificationStatus,
            verify_metadata,
        };

        // Generate fingerprint (blocking operation)
        let path = std::path::PathBuf::from(&track.path);
        let fp_result =
            tokio::task::spawn_blocking(move || fingerprint::generate_fingerprint(&path))
                .await
                .ok()?
                .ok()?;

        // Query AcoustID
        let api_key = std::env::var("ACOUSTID_API_KEY").ok()?;
        let client = acoustid::AcoustIdClient::new(&api_key);
        let identifications = client.lookup(&fp_result).await.ok()?;

        // Build existing metadata
        let existing = ExistingMetadata {
            title: Some(track.title.clone()),
            artist: if track.artist_name != "Unknown Artist" {
                Some(track.artist_name.clone())
            } else {
                None
            },
            album: if track.album_name != "Unknown Album" {
                Some(track.album_name.clone())
            } else {
                None
            },
            year: track.year.map(|y| y as i32),
            track_number: track.track_number.map(|t| t as u32),
            musicbrainz_recording_id: None, // Would need to track this in DB
        };

        // Convert TrackIdentification results to FingerprintMatch
        let matches: Vec<FingerprintMatch> = identifications
            .iter()
            .filter(|id| id.score > 0.5)
            .map(|id| {
                // Build release info from the identified track
                let releases: Vec<ReleaseInfo> = if let (Some(release_id), Some(album)) =
                    (id.track.release_id.clone(), id.track.album.clone())
                {
                    vec![ReleaseInfo {
                        release_id,
                        title: album,
                        year: id.track.year,
                        release_type: id
                            .track
                            .release_type
                            .as_ref()
                            .map(|t| ReleaseType::parse(t))
                            .unwrap_or_default(),
                        track_number: id.track.track_number,
                        album_match_score: 0.0,
                    }]
                } else {
                    vec![]
                };

                FingerprintMatch {
                    confidence: id.score,
                    recording_id: id.track.recording_id.clone().unwrap_or_default(),
                    title: id.track.title.clone().unwrap_or_default(),
                    artist: id.track.artist.clone().unwrap_or_default(),
                    releases: releases.clone(),
                    best_release: releases.into_iter().next(),
                }
            })
            .collect();

        if matches.is_empty() {
            return Some(crate::health::VerificationResult {
                status: VerificationStatus::NoMatch,
                existing,
                best_match: None,
                alternatives: vec![],
                issues: vec![],
            });
        }

        // Verify against best match
        Some(verify_metadata(&existing, &matches))
    }

    /// Apply verification results to quality assessment.
    fn apply_verification_to_quality(
        &self,
        mut quality: TrackQuality,
        verification: &crate::health::VerificationResult,
    ) -> TrackQuality {
        use crate::health::{QualityFlags, VerificationIssue, VerificationStatus};

        // Update confidence and MusicBrainz ID from best match
        if let Some(ref best) = verification.best_match {
            quality.confidence = Some(best.confidence);
            quality.musicbrainz_id = Some(best.recording_id.clone());

            // Clear the "no musicbrainz id" flag since we found one
            quality.flags.remove(QualityFlags::NO_MUSICBRAINZ_ID);
        }

        // Set flags based on verification status
        match verification.status {
            VerificationStatus::Verified => {
                quality.flags.insert(QualityFlags::VERIFIED);
                // Boost score for verified tracks
                quality.score = quality.score.saturating_add(10).min(100);
            }
            VerificationStatus::PartialMatch => {
                // Check for album mismatch (might be compilation)
                if verification
                    .issues
                    .iter()
                    .any(|i| matches!(i, VerificationIssue::AlbumMismatch { .. }))
                {
                    quality.flags.insert(QualityFlags::ALBUM_MISMATCH);
                }
            }
            VerificationStatus::Mismatch => {
                quality.flags.insert(QualityFlags::POSSIBLY_MISLABELED);
                // Penalize score for mislabeled tracks
                quality.score = quality.score.saturating_sub(20);
            }
            VerificationStatus::NoMatch => {
                quality.flags.insert(QualityFlags::UNIDENTIFIED);
            }
            VerificationStatus::Error | VerificationStatus::Pending => {
                // Don't modify flags for errors
            }
        }

        // Set specific mismatch flags
        for issue in &verification.issues {
            match issue {
                VerificationIssue::TitleMismatch { .. } => {
                    quality.flags.insert(QualityFlags::TITLE_MISMATCH);
                }
                VerificationIssue::ArtistMismatch { .. } => {
                    quality.flags.insert(QualityFlags::ARTIST_MISMATCH);
                }
                VerificationIssue::AlbumMismatch { .. } => {
                    quality.flags.insert(QualityFlags::ALBUM_MISMATCH);
                }
                VerificationIssue::BetterAlbumAvailable { .. } => {
                    quality.flags.insert(QualityFlags::BETTER_MATCH_AVAILABLE);
                }
                VerificationIssue::AmbiguousMatch { .. } => {
                    quality.flags.insert(QualityFlags::AMBIGUOUS_MATCH);
                }
                VerificationIssue::LowConfidence { .. } => {
                    quality.flags.insert(QualityFlags::LOW_CONFIDENCE);
                }
                _ => {}
            }
        }

        // Check for multiple albums (compilation candidate)
        if let Some(ref best) = verification.best_match
            && best.releases.len() > 1
        {
            quality.flags.insert(QualityFlags::MULTI_ALBUM);
        }

        quality
    }

    /// Emit an event to listeners.
    async fn emit(&self, event: GardenerEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

/// Assess quality for a single track immediately (utility function).
///
/// This is useful for assessing new tracks as they're scanned.
pub fn assess_track_quality(track: &TrackWithMetadata) -> TrackQuality {
    let filename = std::path::Path::new(&track.path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    let artist = if track.artist_name == "Unknown Artist" {
        None
    } else {
        Some(track.artist_name.as_str())
    };
    let album = if track.album_name == "Unknown Album" {
        None
    } else {
        Some(track.album_name.as_str())
    };

    assess_quality(
        &track.title,
        artist,
        album,
        track.year,
        track.track_number,
        filename,
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::QualityFlags;

    #[test]
    fn test_default_config() {
        let config = GardenerConfig::default();
        assert_eq!(config.batch_size, 10);
        assert!(!config.enable_fingerprinting);
    }

    #[test]
    fn test_assess_track_quality() {
        let track = TrackWithMetadata {
            id: 1,
            title: "Bohemian Rhapsody".to_string(),
            path: "/music/queen/bohemian_rhapsody.mp3".to_string(),
            duration: Some(354),
            track_number: Some(11),
            artist_name: "Queen".to_string(),
            album_name: "A Night at the Opera".to_string(),
            year: Some(1975),
            quality_score: None,
            quality_flags: None,
        };

        let quality = assess_track_quality(&track);
        // Should have good quality since all metadata is present
        // Only missing MusicBrainz ID and fingerprint confidence
        assert!(quality.score >= 70);
        assert!(quality.flags.contains(QualityFlags::NO_MUSICBRAINZ_ID));
        assert!(quality.flags.contains(QualityFlags::NEVER_CHECKED));
    }

    #[test]
    fn test_assess_poor_quality() {
        let track = TrackWithMetadata {
            id: 1,
            title: "track01".to_string(),
            path: "/music/track01.mp3".to_string(),
            duration: Some(200),
            track_number: None,
            artist_name: "Unknown Artist".to_string(),
            album_name: "Unknown Album".to_string(),
            year: None,
            quality_score: None,
            quality_flags: None,
        };

        let quality = assess_track_quality(&track);
        assert!(quality.score < 50);
        assert!(quality.flags.contains(QualityFlags::MISSING_ARTIST));
        assert!(quality.flags.contains(QualityFlags::MISSING_ALBUM));
        assert!(quality.flags.contains(QualityFlags::TITLE_IS_FILENAME));
    }
}
