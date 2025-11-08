//! Core algorithms and geometry.
//!
//! Cross-refs live in doc comments:
//! TH: anchors refer to docs/src/thesis/*.md headings.
//! VK: UUIDs refer to Vibe Kanban tickets.
//!
//! API Policy
//! - This crate is project-internal. There is no stable public API.
//! - Agents prefer clarity and better design over compatibility; breaking changes
//!   are encouraged when they improve quality and align with tickets/specs.
//! - See AGENTS.md → “API Policy (Internal Only)”.

pub mod api;
pub mod geom2;
pub mod geom4;
pub mod oriented_edge;
pub mod rand4;

/// Library version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Convenience re-exports to align code with the thesis notation.
// TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md (uses Vec2/Mat2/Aff2).
pub use geom2::{Aff1, Aff2, GeomCfg};
pub use nalgebra::{Matrix2 as Mat2, Vector2 as Vec2};

/// Common geometry exports for quick imports in callers.
pub mod prelude {
    pub use crate::geom2::rand::{
        draw_polygon_radial, polar, recenter_rescale, Bounds2, RadialCfg, ReplayToken, VertexCount,
    };
    pub use crate::geom2::{
        fixed_point_in_poly, from_points_convex_hull_strict, rotation_angle, Aff1, Aff2, GeomCfg,
        HalfspaceIntersection, Hs2, Poly2,
    };
    pub use nalgebra::{Matrix2 as Mat2, Vector2 as Vec2};
}

/// Signed area of the parallelogram spanned by vectors `a` and `b` in R².
/// Positive for a→b counterclockwise, negative otherwise. Used by Python bindings.
#[inline]
pub fn parallelogram_area(a: Vec2<f64>, b: Vec2<f64>) -> f64 {
    a.x * b.y - a.y * b.x
}
