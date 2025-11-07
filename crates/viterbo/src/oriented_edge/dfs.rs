//! Depth‑first search with push‑forward pruning and fixed‑point closure.

use nalgebra::{Matrix2, Vector2};

use crate::geom2::{fixed_point_in_poly, ordered::HalfspaceIntersection, Aff1, Aff2, GeomCfg};
use crate::geom4::Poly4;

use super::build::build_graph;
use super::types::{Graph, RidgeId, SearchCfg, State};

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

/// Also build graph internally and return fixed point.
pub fn solve_with_defaults_fp(poly: &mut Poly4) -> Option<(f64, Vec<RidgeId>, Vector2<f64>)> {
    let g = build_graph(poly, GeomCfg::default());
    dfs_solve_with_fp(&g, GeomCfg::default(), SearchCfg::default())
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
            if e.to == state.start {
                // Close cycle with fixed-point solve in the start chart
                if let Some((z, val)) = fixed_point_in_poly(
                    next.phi_start_to_current,
                    &next.candidate,
                    &next.action,
                    self.cfg,
                ) {
                    if val < self.best {
                        self.best = val;
                        self.best_cycle = self.stack.clone();
                        self.best_z = Some(z);
                    }
                }
                continue;
            }
            self.stack.push(e.to);
            self.recur_fp(next);
            self.stack.pop();
        }
    }

    fn recur(&mut self, state: State) {
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
            if e.to == state.start {
                // Close cycle with fixed-point solve in the start chart
                if let Some((_z, val)) = fixed_point_in_poly(
                    next.phi_start_to_current,
                    &next.candidate,
                    &next.action,
                    self.cfg,
                ) {
                    if val < self.best {
                        self.best = val;
                        self.best_cycle = self.stack.clone();
                    }
                }
                continue;
            }
            self.stack.push(e.to);
            self.recur(next);
            self.stack.pop();
        }
    }
}
