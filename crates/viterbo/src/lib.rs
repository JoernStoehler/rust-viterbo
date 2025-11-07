//! Core algorithms and geometry.
//!
//! Cross-refs live in doc comments:
//! TH: anchors refer to docs/src/thesis/*.md headings.
//! VK: UUIDs refer to Vibe Kanban tickets.

pub mod geom2;
pub mod geom4;

/// Library version string.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Convenience re-exports to align code with the thesis notation.
// TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md (uses Vec2/Mat2/Aff2).
pub use nalgebra::{Matrix2 as Mat2, Vector2 as Vec2};
pub use geom2::{Aff1, Aff2, GeomCfg};

/// Common geometry exports for quick imports in callers.
pub mod prelude {
    pub use nalgebra::{Matrix2 as Mat2, Vector2 as Vec2};
    pub use crate::geom2::{
        fixed_point_in_poly, rotation_angle, Aff1, Aff2, GeomCfg, HalfspaceIntersection, Hs2, Poly2,
        from_points_convex_hull_strict,
    };
    pub use crate::geom2::rand::{draw_polygon_radial, polar, recenter_rescale, Bounds2, RadialCfg, ReplayToken, VertexCount};
}
