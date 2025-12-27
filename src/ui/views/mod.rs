//! View rendering functions for the UI components.
//!
//! This module is organized into submodules by concern:
//! - `layout`: Main layout composition (sidebar, panes)
//! - `player`: Player controls and visualization
//! - `library`: Library pane with track list and organization
//! - `settings`: Settings pane with organized sections
//! - `enrich`: Batch enrichment pane
//! - `diagnostics`: System diagnostics view
//! - `track_detail`: Track detail modal

mod diagnostics_view;
mod enrich;
pub mod helpers;
mod layout;
mod library;
mod player;
mod settings;
mod track_detail;

pub use layout::loaded_view;
