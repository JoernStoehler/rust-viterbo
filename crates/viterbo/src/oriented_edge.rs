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
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md (sections “Face Graphs”, “Per‑edge maps ψ_ij, τ‑inequalities, A_ij”, “Search and Pruning (push‑forward)”, and the fixed‑point closure.)
//! - Code cross‑refs: `geom2::{Poly2,Hs2,Aff2,Aff1,GeomCfg,rotation_angle,fixed_point_in_poly}`,
//!   `geom4::{Poly4,enumerate_faces_from_h,face2_as_poly2_hrep,oriented_orth_map_face2,reeb_on_facets}`.
//!
//! Notes
//! - Rotation pruning is optional. We expose a search config knob and default
//!   to “off” to keep smoke tests fast and deterministic while the background
//!   normalization is finalized per the thesis.
//! - Determinism note: Because ρ is read from the polar factor of Dψ (via SVD),
//!   numerical differences across compiler flags/CPU features/BLAS paths may
//!   change pruning near the ρ=2 boundary and thus the DFS node counts, even if
//!   the final minimizing cycle and action remain the same. This variability is
//!   acceptable; use the config switch only for controlled ablations.

use nalgebra::{Matrix2, Matrix2x4, Matrix4x2, Vector2};

use crate::geom2::{
    ordered::HalfspaceIntersection, rotation_angle, Aff1, Aff2, GeomCfg, Hs2, Poly2,
};
use crate::geom4::{
    enumerate_faces_from_h, face2_as_poly2_hrep, oriented_orth_map_face2, reeb_on_facets, Poly4,
};

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

/// Build the oriented-edge graph (ridges as nodes; facet‑labeled edges).
///
/// TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md (Face Graphs; Per‑edge maps)
pub fn build_graph(poly: &mut Poly4, cfg: GeomCfg) -> Graph {
    // Enumerate faces once.
    let (_v_all, _edges1, faces2, _facets3) = enumerate_faces_from_h(&poly.h);
    let num_facets = poly.h.len();
    // Build ridge data with charts and strict polygons.
    let mut ridges: Vec<Ridge> = Vec::new();
    let mut by_facet: Vec<Vec<RidgeId>> = vec![Vec::new(); num_facets];
    for (ridx, f2) in faces2.iter().enumerate() {
        let (fi, fj) = f2.facets;
        // Orientation choice: positive; we keep a single canonical chart.
        let (chart_u, chart_ut) =
            oriented_orth_map_face2(&poly.h, fi, fj, true).expect("face chart");
        let poly2 = face2_as_poly2_hrep(poly, fi, fj, true).expect("face poly2");
        let node = Ridge {
            facets: (FacetId(fi), FacetId(fj)),
            poly: poly2,
            chart_u,
            chart_ut,
        };
        let id = RidgeId(ridx);
        ridges.push(node);
        by_facet[fi].push(id);
        by_facet[fj].push(id);
    }
    // Precompute facet Reeb directions v_F = J n_F (no normalization required).
    let v_f = reeb_on_facets(&poly.h);

    // Helper closures
    let other_facet = |r: &Ridge, f: usize| -> usize {
        let (a, b) = (r.facets.0 .0, r.facets.1 .0);
        if a == f {
            b
        } else {
            a
        }
    };
    let hs = &poly.h;

    // Build edges per facet using τ-inequality construction on the current ridge chart.
    let mut edges: Vec<EdgeData> = Vec::new();
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); ridges.len()];
    for f in 0..num_facets {
        let ridges_in_f = &by_facet[f];
        if ridges_in_f.len() < 2 {
            continue;
        }
        let v = v_f[f];
        // For each ordered pair (i,j) inside facet f build ψ_ij and dom inequalities.
        for &ri in ridges_in_f {
            for &rj in ridges_in_f {
                if ri == rj {
                    continue;
                }
                let node_i = &ridges[ri.0];
                let node_j = &ridges[rj.0];
                // Identify the cofacets H_i and H_j (the facet in the pair not equal to f).
                let h_idx_j = other_facet(node_j, f);
                // Only edges whose forward denominator is positive are admissible.
                let d_j = hs[h_idx_j].n.dot(&v);
                if d_j <= cfg.eps_tau {
                    continue;
                }
                // Build domain as A_i intersect all τ_j <= τ_k for admissible k (d_k>0).
                let mut dom = node_i.poly.clone();
                // τ_j > 0 ⇒ b_Hj - <n_Hj, x> > 0. In y-coordinates (chart of i):
                // a_pos · y <= b_Hj - eps (we use a tiny slack for strictness).
                let a_pos = node_i.chart_ut.transpose() * hs[h_idx_j].n;
                let c_pos = hs[h_idx_j].c - cfg.eps_feas;
                dom.insert_halfspace(Hs2::new(Vector2::new(a_pos[0], a_pos[1]), c_pos));
                // τ_j <= τ_k inequalities for all k with d_k>0 (still inside facet f).
                for &rk in ridges_in_f {
                    let h_idx_k = other_facet(&ridges[rk.0], f);
                    if h_idx_k == h_idx_j {
                        continue;
                    }
                    let d_k = hs[h_idx_k].n.dot(&v);
                    if d_k <= cfg.eps_tau {
                        continue; // not forward‑hitting
                    }
                    // Inequality: d_k(b_j - n_j·x) <= d_j(b_k - n_k·x)
                    // Rearranged in y (x = U_i^T y): (d_k n_j - d_j n_k)·U_i^T y <= d_j b_k - d_k b_j
                    let coeff_4 = hs[h_idx_j].n * d_k - hs[h_idx_k].n * d_j;
                    let a2 = node_i.chart_ut.transpose() * coeff_4;
                    let c2 = d_j * hs[h_idx_k].c - d_k * hs[h_idx_j].c;
                    dom.insert_halfspace(Hs2::new(Vector2::new(a2[0], a2[1]), c2));
                }
                // Quick conservative emptiness check; skip edge if empty.
                if dom.halfspace_intersection_eps(cfg.eps_feas).is_empty() {
                    continue;
                }
                // Build ψ_ij on charts (matrix and translation) and A_ij.
                // y_j = U_j (U_i^T y_i + τ(y_i) v_f), τ(y_i) = (b_Hj - n_Hj·U_i^T y) / d_j
                let u_j = node_j.chart_u;
                let ut_i = node_i.chart_ut;
                let u_vec = u_j * v;
                let r_row = hs[h_idx_j].n.transpose() * ut_i; // 1x2
                                                              // M = U_j U_i^T - (u_vec * r_row)/d_j  (rank‑1 outer product)
                let u_outer = Matrix2::new(
                    u_vec[(0, 0)] * r_row[(0, 0)],
                    u_vec[(0, 0)] * r_row[(0, 1)],
                    u_vec[(1, 0)] * r_row[(0, 0)],
                    u_vec[(1, 0)] * r_row[(0, 1)],
                ) * (1.0 / d_j);
                let m = (u_j * ut_i) - u_outer;
                let t = Vector2::new(
                    u_vec[(0, 0)] * (hs[h_idx_j].c / d_j),
                    u_vec[(1, 0)] * (hs[h_idx_j].c / d_j),
                );
                let map_ij = Aff2 { m, t };
                let rotation_inc = rotation_angle(&map_ij).unwrap_or(0.0);
                // A_ij(y) = (b_F/(2 d_j)) b_Hj - (b_F/(2 d_j)) (n_Hj·U_i^T) y
                let bf = hs[f].c;
                let a_vec = ut_i.transpose() * hs[h_idx_j].n;
                let action_inc = Aff1 {
                    a: -a_vec * (bf / (2.0 * d_j)),
                    b: (bf / (2.0 * d_j)) * hs[h_idx_j].c,
                };
                // Image polygon to help downstream sanity checks/visuals.
                let img = dom.push_forward(&map_ij).unwrap_or_default();
                // Per-edge lower bound of A_inc over dom by checking HPI vertices.
                let lb = match dom.halfspace_intersection() {
                    HalfspaceIntersection::Bounded(verts) => verts
                        .into_iter()
                        .map(|z| action_inc.eval(z))
                        .fold(f64::INFINITY, f64::min),
                    _ => f64::NEG_INFINITY, // unbounded; treat as very small to explore late
                };
                let eidx = edges.len();
                edges.push(EdgeData {
                    from: ri,
                    to: rj,
                    facet: FacetId(f),
                    dom_in: dom,
                    img_out: img,
                    map_ij,
                    action_inc,
                    rotation_inc,
                    lb_action: lb,
                });
                adj[ri.0].push(eidx);
            }
        }
    }
    // Sort adjacency by per-edge lower bounds (ascending).
    for out in adj.iter_mut() {
        out.sort_by(|&a, &b| {
            edges[a]
                .lb_action
                .partial_cmp(&edges[b].lb_action)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    Graph {
        ridges,
        edges,
        adj,
        num_facets,
    }
}

/// Solve oriented‑edge DFS with incumbent, push‑forward pruning, and fixed‑point closure.
///
/// Returns the best action value found (if any) and the ridge cycle (by ids).
pub fn dfs_solve(graph: &Graph, cfg: GeomCfg, scfg: SearchCfg) -> Option<(f64, Vec<RidgeId>)> {
    DfsRunner::new(graph, cfg, scfg).solve()
}

/// Convenience: build graph and solve with default tolerances and pruning.
pub fn solve_with_defaults(poly: &mut Poly4) -> Option<(f64, Vec<RidgeId>)> {
    let g = build_graph(poly, GeomCfg::default());
    dfs_solve(&g, GeomCfg::default(), SearchCfg::default())
}

/// Solve and also return the fixed point in the start chart.
pub fn dfs_solve_with_fp(
    graph: &Graph,
    cfg: GeomCfg,
    scfg: SearchCfg,
) -> Option<(f64, Vec<RidgeId>, Vector2<f64>)> {
    DfsRunner::new(graph, cfg, scfg).solve_with_fp()
}

/// DFS runner carrying shared context and accumulators.
struct DfsRunner<'a> {
    g: &'a Graph,
    cfg: GeomCfg,
    scfg: SearchCfg,
    best: f64,
    best_cycle: Vec<RidgeId>,
    best_z: Option<Vector2<f64>>,
    stack: Vec<RidgeId>,
}

impl<'a> DfsRunner<'a> {
    fn new(g: &'a Graph, cfg: GeomCfg, scfg: SearchCfg) -> Self {
        Self {
            g,
            cfg,
            scfg,
            best: f64::INFINITY,
            best_cycle: Vec::new(),
            best_z: None,
            stack: Vec::new(),
        }
    }

    fn solve(&mut self) -> Option<(f64, Vec<RidgeId>)> {
        let n = self.g.ridges.len();
        for s in 0..n {
            let start = RidgeId(s);
            let state0 = State {
                start,
                cur: start,
                facets_seen: vec![false; self.g.num_facets],
                candidate: self.g.ridges[s].poly.clone(),
                action: Aff1 {
                    a: Vector2::new(0.0, 0.0),
                    b: 0.0,
                },
                rho: 0.0,
                phi_start_to_current: Aff2 {
                    m: Matrix2::identity(),
                    t: Vector2::new(0.0, 0.0),
                },
            };
            self.stack.push(start);
            self.recur(state0);
            self.stack.clear();
        }
        if self.best.is_finite() {
            Some((self.best, self.best_cycle.clone()))
        } else {
            None
        }
    }

    fn solve_with_fp(&mut self) -> Option<(f64, Vec<RidgeId>, Vector2<f64>)> {
        let n = self.g.ridges.len();
        for s in 0..n {
            let start = RidgeId(s);
            let state0 = State {
                start,
                cur: start,
                facets_seen: vec![false; self.g.num_facets],
                candidate: self.g.ridges[s].poly.clone(),
                action: Aff1 {
                    a: Vector2::new(0.0, 0.0),
                    b: 0.0,
                },
                rho: 0.0,
                phi_start_to_current: Aff2 {
                    m: Matrix2::identity(),
                    t: Vector2::new(0.0, 0.0),
                },
            };
            self.stack.push(start);
            self.recur_fp(state0);
            self.stack.clear();
        }
        if self.best.is_finite() {
            Some((self.best, self.best_cycle.clone(), self.best_z.unwrap()))
        } else {
            None
        }
    }

    fn recur_fp(&mut self, state: State) {
        if let HalfspaceIntersection::Bounded(verts) = state.candidate.halfspace_intersection() {
            let cur_lb = verts
                .into_iter()
                .map(|z| state.action.eval(z))
                .fold(f64::INFINITY, f64::min);
            if cur_lb >= self.best - 1e-12 {
                return;
            }
        }
        let out_edges = &self.g.adj[state.cur.0];
        for &eidx in out_edges {
            let e = &self.g.edges[eidx];
            if state.facets_seen[e.facet.0] {
                continue;
            }
            let c_dom = state.candidate.intersect(&e.dom_in);
            if c_dom
                .halfspace_intersection_eps(self.cfg.eps_feas)
                .is_empty()
            {
                continue;
            }
            let c1 = if let Some(p) = c_dom.push_forward(&e.map_ij) {
                p
            } else {
                continue;
            };
            let rho1 = state.rho + e.rotation_inc;
            if self.scfg.use_rotation_prune && rho1 > self.scfg.rotation_budget {
                continue;
            }
            let a_pull = if let Some(a1) = state.action.compose_with_inv_affine2(&e.map_ij) {
                a1
            } else {
                continue;
            };
            let a_edge = if let Some(a2) = e.action_inc.compose_with_inv_affine2(&e.map_ij) {
                a2
            } else {
                continue;
            };
            let a1 = a_pull.add(&a_edge);
            let c2 = c1.with_cut(a1.to_cut(self.best));
            if c2.halfspace_intersection_eps(self.cfg.eps_feas).is_empty() {
                continue;
            }
            let phi1 = Aff2 {
                m: e.map_ij.m * state.phi_start_to_current.m,
                t: e.map_ij.m * state.phi_start_to_current.t + e.map_ij.t,
            };
            let mut next_seen = state.facets_seen.clone();
            next_seen[e.facet.0] = true;
            let next = State {
                start: state.start,
                cur: e.to,
                facets_seen: next_seen,
                candidate: c2,
                action: a1,
                rho: rho1,
                phi_start_to_current: phi1,
            };
            self.stack.push(e.to);
            if e.to == state.start {
                if let Some((z_star, a_val)) = crate::geom2::fixed_point_in_poly(
                    next.phi_start_to_current,
                    &next.candidate,
                    &next.action,
                    self.cfg,
                ) {
                    if a_val < self.best {
                        self.best = a_val;
                        self.best_cycle = self.stack.clone();
                        self.best_z = Some(z_star);
                    }
                }
            } else {
                self.recur_fp(next);
            }
            self.stack.pop();
        }
    }

    fn recur(&mut self, state: State) {
        // Action lower bound on current candidate set: quick prune using polygon vertices.
        if let HalfspaceIntersection::Bounded(verts) = state.candidate.halfspace_intersection() {
            let cur_lb = verts
                .into_iter()
                .map(|z| state.action.eval(z))
                .fold(f64::INFINITY, f64::min);
            if cur_lb >= self.best - 1e-12 {
                return;
            }
        }
        let out_edges = &self.g.adj[state.cur.0];
        for &eidx in out_edges {
            let e = &self.g.edges[eidx];
            // No‑revisit pruning by facets (HK simple-loop): do not repeat the traversed facet.
            if state.facets_seen[e.facet.0] {
                continue;
            }
            // Intersect candidate with dom_i.
            let c_dom = state.candidate.intersect(&e.dom_in);
            if c_dom
                .halfspace_intersection_eps(self.cfg.eps_feas)
                .is_empty()
            {
                continue;
            }
            // Push forward.
            let c1 = if let Some(p) = c_dom.push_forward(&e.map_ij) {
                p
            } else {
                continue;
            };
            // Rotation prune (optional).
            let rho1 = state.rho + e.rotation_inc;
            if self.scfg.use_rotation_prune && rho1 > self.scfg.rotation_budget {
                continue;
            }
            // Update action as A' = A∘ψ^{-1} + A_inc∘ψ^{-1}.
            let a_pull = if let Some(a1) = state.action.compose_with_inv_affine2(&e.map_ij) {
                a1
            } else {
                continue;
            };
            let a_edge = if let Some(a2) = e.action_inc.compose_with_inv_affine2(&e.map_ij) {
                a2
            } else {
                continue;
            };
            let a1 = a_pull.add(&a_edge);
            // Cut by incumbent bound.
            let c2 = c1.with_cut(a1.to_cut(self.best));
            if c2.halfspace_intersection_eps(self.cfg.eps_feas).is_empty() {
                continue;
            }
            // Compose forward mapping (start -> new current).
            let phi1 = Aff2 {
                m: e.map_ij.m * state.phi_start_to_current.m,
                t: e.map_ij.m * state.phi_start_to_current.t + e.map_ij.t,
            };
            // Advance.
            let mut next_seen = state.facets_seen.clone();
            next_seen[e.facet.0] = true;
            let next = State {
                start: state.start,
                cur: e.to,
                facets_seen: next_seen,
                candidate: c2,
                action: a1,
                rho: rho1,
                phi_start_to_current: phi1,
            };
            self.stack.push(e.to);
            // Close cycle
            if e.to == state.start {
                // Fixed‑point on start chart; candidate already in start chart coordinates.
                if let Some((_, a_val)) = crate::geom2::fixed_point_in_poly(
                    next.phi_start_to_current,
                    &next.candidate,
                    &next.action,
                    self.cfg,
                ) {
                    if a_val < self.best {
                        self.best = a_val;
                        self.best_cycle = self.stack.clone();
                    }
                }
            } else {
                self.recur(next);
            }
            self.stack.pop();
        }
    }
}

/// Build graph and solve, returning the fixed point as well (defaults).
pub fn solve_with_defaults_fp(poly: &mut Poly4) -> Option<(f64, Vec<RidgeId>, Vector2<f64>)> {
    let g = build_graph(poly, GeomCfg::default());
    dfs_solve_with_fp(&g, GeomCfg::default(), SearchCfg::default())
}
// legacy free function removed in favor of DfsRunner

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom4::Hs4;
    use nalgebra::Vector4;

    fn cube4_hs(side: f64) -> Vec<Hs4> {
        let s = side;
        let axes = [
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        let mut hs = Vec::new();
        for a in axes {
            hs.push(Hs4::new(a, s));
            hs.push(Hs4::new(-a, s));
        }
        hs
    }

    #[test]
    fn smoke_graph_build_cube_edges_exist() {
        // Build a simple hypercube [-1,1]^4 and ensure we get at least some oriented edges.
        let mut p4 = Poly4::from_h(cube4_hs(1.0));
        let g = build_graph(&mut p4, GeomCfg::default());
        assert!(!g.ridges.is_empty());
        assert!(!g.edges.is_empty());
        // Pick a facet (x1=+1, index 0 by construction) and ensure it has outgoing edges.
        let ridges_in_f0: Vec<_> = g
            .ridges
            .iter()
            .enumerate()
            .filter(|(_, r)| r.facets.0 .0 == 0 || r.facets.1 .0 == 0)
            .map(|(i, _)| RidgeId(i))
            .collect();
        assert!(ridges_in_f0.len() >= 2);
        let out_any: usize = ridges_in_f0.iter().map(|rid| g.adj[rid.0].len()).sum();
        assert!(out_any > 0);
        // Check that at least one edge has non-empty dom and img.
        let has_valid = g.edges.iter().any(|e| {
            !e.dom_in.halfspace_intersection_eps(1e-9).is_empty()
                && !e.img_out.halfspace_intersection_eps(1e-9).is_empty()
        });
        assert!(has_valid);
    }

    #[test]
    fn smoke_dfs_trivial_cycle_manual_identity() {
        // Synthetic tiny graph: two ridges with identity mappings back and forth.
        // This validates fixed‑point closure and action accumulation machinery.
        let poly_unit = {
            let mut p = Poly2::default();
            // Square: |x|<=1, |y|<=1
            p.insert_halfspace(Hs2::new(Vector2::new(1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(-1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, 1.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, -1.0), 1.0));
            p
        };
        // Dummy charts (only used to satisfy struct fields; identity everywhere).
        let u = Matrix2x4::zeros();
        let ut = Matrix4x2::zeros();
        let r0 = Ridge {
            facets: (FacetId(0), FacetId(1)),
            poly: poly_unit.clone(),
            chart_u: u,
            chart_ut: ut,
        };
        let r1 = Ridge {
            facets: (FacetId(1), FacetId(2)),
            poly: poly_unit.clone(),
            chart_u: u,
            chart_ut: ut,
        };
        let ridges = vec![r0, r1];
        let id = Aff2 {
            m: Matrix2::identity(),
            t: Vector2::new(0.0, 0.0),
        };
        let zero = Aff1 {
            a: Vector2::new(0.0, 0.0),
            b: 0.0,
        };
        let e01 = EdgeData {
            from: RidgeId(0),
            to: RidgeId(1),
            facet: FacetId(10),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: id,
            action_inc: zero,
            rotation_inc: 0.0,
            lb_action: 0.0,
        };
        let e10 = EdgeData {
            from: RidgeId(1),
            to: RidgeId(0),
            facet: FacetId(11),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: id,
            action_inc: zero,
            rotation_inc: 0.0,
            lb_action: 0.0,
        };
        let edges = vec![e01, e10];
        let g = Graph {
            ridges,
            edges,
            adj: vec![vec![0], vec![1]],
            num_facets: 12,
        };
        let res = dfs_solve(&g, GeomCfg::default(), SearchCfg::default());
        assert!(res.is_some());
        let (best, cyc) = res.unwrap();
        assert!(best.abs() < 1e-9);
        assert!(!cyc.is_empty());
    }

    #[test]
    fn rotation_prune_blocks_high_rho_cycle_but_allows_when_disabled() {
        // Build a 3‑ridge cycle with per‑edge rotation ≈ 0.8 (in units of π),
        // so total ρ ≈ 2.4 > 2. This should be pruned by default, and accepted
        // if rotation pruning is disabled.
        let poly_unit = {
            let mut p = Poly2::default();
            p.insert_halfspace(Hs2::new(Vector2::new(1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(-1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, 1.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, -1.0), 1.0));
            p
        };
        let u = Matrix2x4::zeros();
        let ut = Matrix4x2::zeros();
        let ridges = vec![
            Ridge {
                facets: (FacetId(0), FacetId(1)),
                poly: poly_unit.clone(),
                chart_u: u,
                chart_ut: ut,
            },
            Ridge {
                facets: (FacetId(1), FacetId(2)),
                poly: poly_unit.clone(),
                chart_u: u,
                chart_ut: ut,
            },
            Ridge {
                facets: (FacetId(2), FacetId(0)),
                poly: poly_unit.clone(),
                chart_u: u,
                chart_ut: ut,
            },
        ];
        // Rotation by θ = 0.8π -> rho = 0.8 per edge.
        let theta = 0.8 * std::f64::consts::PI;
        let rot = Matrix2::new(theta.cos(), -theta.sin(), theta.sin(), theta.cos());
        let psi_rot = Aff2 {
            m: rot,
            t: Vector2::new(0.0, 0.0),
        };
        // zero action
        let zero = Aff1 {
            a: Vector2::new(0.0, 0.0),
            b: 0.0,
        };
        let e01 = EdgeData {
            from: RidgeId(0),
            to: RidgeId(1),
            facet: FacetId(10),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: psi_rot,
            action_inc: zero,
            rotation_inc: rotation_angle(&psi_rot).unwrap(),
            lb_action: 0.0,
        };
        let e12 = EdgeData {
            from: RidgeId(1),
            to: RidgeId(2),
            facet: FacetId(11),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: psi_rot,
            action_inc: zero,
            rotation_inc: rotation_angle(&psi_rot).unwrap(),
            lb_action: 0.0,
        };
        let e20 = EdgeData {
            from: RidgeId(2),
            to: RidgeId(0),
            facet: FacetId(12),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: psi_rot,
            action_inc: zero,
            rotation_inc: rotation_angle(&psi_rot).unwrap(),
            lb_action: 0.0,
        };
        let edges = vec![e01, e12, e20];
        let adj = vec![vec![0usize], vec![1usize], vec![2usize]];
        let g = Graph {
            ridges,
            edges,
            adj,
            num_facets: 16,
        };
        // Default config has rotation pruning enabled: expect None.
        let res_pruned = dfs_solve(&g, GeomCfg::default(), SearchCfg::default());
        assert!(res_pruned.is_none());
        // Disable rotation pruning: expect a valid zero‑action cycle.
        let cfg_off = SearchCfg {
            use_rotation_prune: false,
            rotation_budget: 2.0,
        };
        let res_ok = dfs_solve(&g, GeomCfg::default(), cfg_off);
        assert!(res_ok.is_some());
        let (best, _cyc) = res_ok.unwrap();
        assert!(best.abs() < 1e-9);
    }

    #[test]
    fn dfs_with_fp_returns_fixed_point_on_synthetic_identity_cycle() {
        // Two‑ridge identity cycle; fixed point exists (any point). We just assert we get one
        // and that it lies inside the unit square bounds.
        let poly_unit = {
            let mut p = Poly2::default();
            p.insert_halfspace(Hs2::new(Vector2::new(1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(-1.0, 0.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, 1.0), 1.0));
            p.insert_halfspace(Hs2::new(Vector2::new(0.0, -1.0), 1.0));
            p
        };
        let u = Matrix2x4::zeros();
        let ut = Matrix4x2::zeros();
        let ridges = vec![
            Ridge {
                facets: (FacetId(0), FacetId(1)),
                poly: poly_unit.clone(),
                chart_u: u,
                chart_ut: ut,
            },
            Ridge {
                facets: (FacetId(1), FacetId(2)),
                poly: poly_unit.clone(),
                chart_u: u,
                chart_ut: ut,
            },
        ];
        let id = Aff2 {
            m: Matrix2::identity(),
            t: Vector2::new(0.0, 0.0),
        };
        let zero = Aff1 {
            a: Vector2::new(0.0, 0.0),
            b: 0.0,
        };
        let e01 = EdgeData {
            from: RidgeId(0),
            to: RidgeId(1),
            facet: FacetId(10),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: id,
            action_inc: zero,
            rotation_inc: 0.0,
            lb_action: 0.0,
        };
        let e10 = EdgeData {
            from: RidgeId(1),
            to: RidgeId(0),
            facet: FacetId(11),
            dom_in: poly_unit.clone(),
            img_out: poly_unit.clone(),
            map_ij: id,
            action_inc: zero,
            rotation_inc: 0.0,
            lb_action: 0.0,
        };
        let edges = vec![e01, e10];
        let adj = vec![vec![0usize], vec![1usize]];
        let g = Graph {
            ridges,
            edges,
            adj,
            num_facets: 16,
        };
        let res = dfs_solve_with_fp(&g, GeomCfg::default(), SearchCfg::default());
        assert!(res.is_some());
        let (best, _cycle, z) = res.unwrap();
        assert!(best.abs() < 1e-9);
        assert!(z.x <= 1.0 + 1e-9 && z.x >= -1.0 - 1e-9);
        assert!(z.y <= 1.0 + 1e-9 && z.y >= -1.0 - 1e-9);
    }
}
