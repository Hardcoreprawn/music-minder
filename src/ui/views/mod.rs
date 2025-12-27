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
//! - `toast`: Toast notifications
//! - `loading`: Loading states with fun messages

mod diagnostics_view;
mod enrich;
pub mod helpers;
mod layout;
mod library;
pub mod loading;
mod player;
mod settings;
pub mod toast;
mod track_detail;

pub use layout::loaded_view;
pub use toast::ToastQueue;
