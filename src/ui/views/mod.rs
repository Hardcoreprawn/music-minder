//! View rendering functions for the UI components.
//!
//! This module is organized into submodules by concern:
//! - `layout`: Main layout composition (sidebar, panes)
//! - `player`: Player controls and visualization
//! - `library`: Library pane with track list and organization
//! - `diagnostics`: System diagnostics view

mod layout;
mod player;
mod library;
mod diagnostics_view;
mod helpers;

pub use layout::loaded_view;
