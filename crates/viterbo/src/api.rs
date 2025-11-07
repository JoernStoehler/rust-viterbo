//! Curated internal API for agents (UNSTABLE).
//!
//! Important
//! - This is not a public API. It is a convenience surface for project-internal
//!   code and tickets. Breaking changes are allowed and expected.
//! - Prefer these re-exports for clarity and consistency across experiments.
//!
//! See AGENTS.md → “API Policy (Internal Only)”.

// 2D strict geometry
pub use crate::geom2::{
    fixed_point_in_poly, from_points_convex_hull_strict, rotation_angle, Aff1, Aff2, GeomCfg, Hs2,
    Poly2,
};
// 2D random polygons
pub use crate::geom2::rand::{
    draw_polygon_radial, polar as poly2_polar, recenter_rescale, Bounds2 as Bounds2D, RadialCfg,
    ReplayToken as Poly2Replay, VertexCount,
};
// 4D polytopes
pub use crate::geom4::{
    enumerate_faces_from_h, face2_as_poly2_hrep, is_symplectic, j_matrix_4, oriented_orth_map_face2,
    reeb_on_facets, Face1, Face2, Face3, Hs4, Poly4,
};
// Oriented-edge algorithm
pub use crate::oriented_edge::{
    build_graph, dfs_solve, dfs_solve_with_fp, solve_with_defaults, solve_with_defaults_fp,
    Affine2 as ChartAff2, Graph, SearchCfg,
};

