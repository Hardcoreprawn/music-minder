//! AcoustID API integration
//!
//! AcoustID is a free service that identifies music by audio fingerprint.
//! API docs: https://acoustid.org/webservice

mod adapter;
mod client;
pub mod dto;

pub use adapter::{best_identification, to_identifications};
pub use client::AcoustIdClient;
