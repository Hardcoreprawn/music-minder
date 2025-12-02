//! Music enrichment module - identifies tracks and fetches metadata from external services.
//!
//! # Architecture
//!
//! This module follows a clean separation between:
//! - **Domain models** (`domain.rs`) - Internal types that represent our business logic
//! - **API DTOs** (`acoustid/dto.rs`, `musicbrainz/dto.rs`) - Exact API response shapes
//! - **Adapters** - Convert DTOs to domain models
//! - **Clients** - HTTP clients for external APIs
//! - **Fingerprint** - Audio fingerprint generation via fpcalc
//! - **Service** - High-level orchestration of the enrichment flow
//!
//! This decoupling means:
//! 1. API changes don't ripple through our codebase
//! 2. We can test API contracts independently
//! 3. We can swap providers without changing business logic
//!
//! # Usage
//!
//! ```ignore
//! use enrichment::{EnrichmentService, EnrichmentConfig};
//!
//! let config = EnrichmentConfig {
//!     acoustid_api_key: "your-api-key".to_string(),
//!     ..Default::default()
//! };
//! let service = EnrichmentService::new(config);
//!
//! // Identify a track
//! let result = service.identify_track(Path::new("song.mp3")).await?;
//! println!("Title: {:?}, Artist: {:?}", result.track.title, result.track.artist);
//! ```

pub mod domain;
pub mod acoustid;
pub mod musicbrainz;
pub mod coverart;
pub mod fingerprint;
pub mod service;

pub use domain::{TrackIdentification, IdentifiedTrack, EnrichmentSource, EnrichmentError, AudioFingerprint};
pub use service::{EnrichmentService, EnrichmentConfig, identify_track};
pub use coverart::{CoverArtClient, CoverArt, CoverSize};

