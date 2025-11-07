//! Oriented-Edge Graph: builders and DFS with push‑forward pruning.
//!
//! Purpose
//! - Construct the 2‑face digraph (ridges as nodes; edges labeled by facets)
//!   from a 4D polytope and provide a depth‑first search that pushes forward
//!   candidate sets, accumulates action and (optionally) rotation, and closes
//!   cycles via a fixed‑point solve.
//!
//! Why this design
//! - Follow the “push‑forward in current ridge chart” formulation for numerical
//!   robustness and simpler composition rules.
//! - Keep the public API minimal and aligned with the thesis notation to
//!   facilitate cross‑checking and future extensions.
//!
//! References
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md (sections “Face Graphs”,
//!   “Per‑edge maps ψ_ij, τ‑inequalities, A_ij”, “Search and Pruning (push‑forward)”,
//!   and the fixed‑point closure.)
//! - Code cross‑refs: `geom2::{Poly2,Hs2,Aff2,Aff1,GeomCfg,rotation_angle,fixed_point_in_poly}`,
//!   `geom4::{Poly4,enumerate_faces_from_h,face2_as_poly2_hrep,oriented_orth_map_face2,reeb_on_facets}`.
//!
//! Note on future maintenance
//! - This module used to be a single file (~800+ LOC). It is now split for
//!   readability: `types.rs` (data types), `build.rs` (graph construction),
//!   and `dfs.rs` (search). Public re-exports preserve the original API.

mod build;
mod dfs;
mod types;

pub use build::build_graph;
pub use dfs::{dfs_solve, dfs_solve_with_fp, solve_with_defaults, solve_with_defaults_fp};
pub use types::{
    Affine2, EdgeData, FacetId, Graph, HPoly2Ordered, Ridge, RidgeId, SearchCfg, State,
};

#[cfg(test)]
mod tests;
