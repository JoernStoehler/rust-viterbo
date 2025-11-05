//! Core algorithms and geometry.
//!
//! Cross-refs live in doc comments:
//! TH: anchors refer to docs/src/thesis/*.md headings.
//! VK: UUIDs refer to Vibe Kanban tickets.

pub mod geometry;

pub use geometry::parallelogram_area;

/// Library version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

