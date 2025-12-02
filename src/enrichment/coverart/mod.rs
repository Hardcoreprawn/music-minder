//! Cover Art Archive integration
//!
//! Fetches album artwork from coverartarchive.org using MusicBrainz release IDs.
//! No API key required.

pub mod dto;
mod client;

pub use client::{CoverArtClient, CoverArt, CoverSize};
