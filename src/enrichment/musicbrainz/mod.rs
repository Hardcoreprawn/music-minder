//! MusicBrainz API integration
//!
//! Provides detailed metadata enrichment by looking up recordings from MusicBrainz.
//! Typically used after AcoustID identifies a recording by its MusicBrainz ID.
//!
//! API docs: https://musicbrainz.org/doc/MusicBrainz_API

mod adapter;
mod client;
pub mod dto;

pub use adapter::to_identification;
pub use client::MusicBrainzClient;
