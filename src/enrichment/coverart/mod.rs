//! Cover Art Archive integration
//!
//! Fetches album artwork from coverartarchive.org using MusicBrainz release IDs.
//! No API key required.

mod client;
pub mod dto;

pub use client::{CoverArt, CoverArtClient, CoverSize};
