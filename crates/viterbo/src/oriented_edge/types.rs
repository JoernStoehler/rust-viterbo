//! Data types for the oriented-edge graph and search state.
//!
//! Kept small and explicit to make `build` and `dfs` modules easy to read.

use nalgebra::{Matrix2x4, Matrix4x2};

use crate::geom2::{Aff1, Aff2, Poly2};

/// Public alias to match thesis/spec naming used across tickets.
pub type HPoly2Ordered = Poly2;
pub type Affine2 = Aff2;

/// Identifier types for clarity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RidgeId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FacetId(pub usize);

/// Ridge node data: facets that define it, its strict polygon in the intrinsic chart,
/// and the linear charts (U, U^T) used by edges.
#[derive(Clone, Debug)]
pub struct Ridge {
    pub facets: (FacetId, FacetId), // unordered pair
    pub poly: HPoly2Ordered,        // source chart polygon A_i
    pub chart_u: Matrix2x4<f64>,    // rows: ON basis of the ridge plane
    pub chart_ut: Matrix4x2<f64>,   // columns: ON basis; acts as left-inverse on-plane
}

/// Per-edge data (i → j inside facet `facet`).
#[derive(Clone, Debug)]
pub struct EdgeData {
    pub from: RidgeId,
    pub to: RidgeId,
    pub facet: FacetId,
    pub dom_in: HPoly2Ordered,
    pub img_out: HPoly2Ordered,
    pub map_ij: Affine2,
    pub action_inc: Aff1,
    pub rotation_inc: f64,
    pub lb_action: f64, // per-edge lower bound on action_inc over dom_in
}

/// Graph of ridges with per-edge maps and bounds; adjacency lists are sorted by
/// increasing `lb_action` to realize “early ordering via per-edge lower bounds”.
#[derive(Clone, Debug)]
pub struct Graph {
    pub ridges: Vec<Ridge>,
    pub edges: Vec<EdgeData>,
    pub adj: Vec<Vec<usize>>, // edge indices out of ridge k (sorted by lb_action)
    pub num_facets: usize,
}

/// Search state carried along DFS (current ridge's chart).
#[derive(Clone, Debug)]
pub struct State {
    pub start: RidgeId,
    pub cur: RidgeId,
    pub facets_seen: Vec<bool>,
    pub candidate: HPoly2Ordered,
    pub action: Aff1,
    pub rho: f64, // accumulated rotation
    /// Forward composition from start chart to current chart.
    /// On closure (cur==start), this is the cycle map on the start chart.
    pub phi_start_to_current: Affine2,
}

/// Search configuration.
#[derive(Clone, Copy, Debug)]
pub struct SearchCfg {
    pub use_rotation_prune: bool,
    /// Theory-fixed budget for 2D rotation accumulation along a cycle.
    /// In 4D for the index-3 minimizer, total ρ ∈ (1,2); we prune when ρ > 2.
    /// Keep configurable only to run controlled ablations/benchmarks.
    pub rotation_budget: f64,
}
impl Default for SearchCfg {
    fn default() -> Self {
        Self {
            // Default ON: rotation pruning is part of the algorithm (not a hyperparameter).
            use_rotation_prune: true,
            rotation_budget: 2.0,
        }
    }
}
