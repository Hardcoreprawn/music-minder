//! AcoustID API integration
//!
//! AcoustID is a free service that identifies music by audio fingerprint.
//! API docs: https://acoustid.org/webservice

pub mod dto;
mod adapter;
mod client;

pub use client::AcoustIdClient;
pub use adapter::{to_identifications, best_identification};
