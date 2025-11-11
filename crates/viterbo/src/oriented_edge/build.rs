//! Graph construction (nodes, edges, bounds) for the oriented-edge algorithm.

use crate::geom2::{
    from_points_convex_hull_strict,
    ordered::{HalfspaceIntersection, Poly2},
    rotation_angle, Aff1, Aff2, GeomCfg, Hs2,
};
use crate::geom4::{
    cfg::TIGHT_EPS, enumerate_faces_from_h, j_matrix_4, oriented_orth_map_face2, reeb_on_facets,
    Poly4,
};
use nalgebra::{Matrix2x4, Matrix4, Vector2, Vector4};

use super::types::{EdgeData, FacetId, Graph, Ridge, RidgeId};

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
    let j = j_matrix_4();
    for f2 in faces2.iter() {
        let (fi, fj) = f2.facets;
        // Canonical chart (ω0-induced orientation); skip Lagrangian faces (ω≈0).
        let Some((chart_u, chart_ut)) = oriented_orth_map_face2(&poly.h, fi, fj) else {
            continue;
        };
        if chart_is_lagrangian(&chart_u, &j) {
            continue;
        }
        let Some(poly2) = ridge_poly_from_vertices(&chart_u, &f2.vertices) else {
            continue;
        };
        let node = Ridge {
            facets: (FacetId(fi), FacetId(fj)),
            poly: poly2,
            chart_u,
            chart_ut,
        };
        let id = RidgeId(ridges.len());
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
                let u_outer = nalgebra::Matrix2::new(
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
                // Orientation should be preserved between canonical ridge charts.
                let det_map = map_ij.m.determinant();
                if !det_map.is_finite() {
                    #[cfg(debug_assertions)]
                    if std::env::var_os("VITERBO_DEBUG_OE").is_some() {
                        eprintln!(
                            "skip ψ_ij with non-finite det (det={det_map}, facet={f}, from={ri:?}{:?}, to={rj:?}{:?}, cofacet_j={h_idx_j}, d_j={d_j})",
                            ridges[ri.0].facets,
                            ridges[rj.0].facets
                        );
                    }
                    continue;
                }
                if det_map.abs() <= cfg.eps_det {
                    #[cfg(debug_assertions)]
                    if std::env::var_os("VITERBO_DEBUG_OE").is_some() {
                        eprintln!(
                            "skip ψ_ij with det≈0 (det={det_map}, facet={f}, from={ri:?}{:?}, to={rj:?}{:?}, cofacet_j={h_idx_j}, d_j={d_j})",
                            ridges[ri.0].facets,
                            ridges[rj.0].facets
                        );
                    }
                    continue;
                }
                if det_map < 0.0 {
                    #[cfg(debug_assertions)]
                    if std::env::var_os("VITERBO_DEBUG_OE").is_some() {
                        eprintln!(
                            "skip ψ_ij with det<0 (det={det_map}, facet={f}, from={ri:?}{:?}, to={rj:?}{:?}, cofacet_j={h_idx_j}, d_j={d_j}, ω_i={}, ω_j={})",
                            ridges[ri.0].facets,
                            ridges[rj.0].facets,
                            chart_signed_omega(&ridges[ri.0].chart_u, &j),
                            chart_signed_omega(&ridges[rj.0].chart_u, &j),
                        );
                    }
                    continue;
                }
                debug_assert!(
                    det_map > 0.0,
                    "ψ_ij must be orientation-preserving between canonical charts (det={det_map}, facet={f}, from={ri:?}{:?}, to={rj:?}{:?}, cofacet_j={h_idx_j}, d_j={d_j})",
                    ridges[ri.0].facets,
                    ridges[rj.0].facets
                );
                let rotation_inc = rotation_angle(&map_ij).unwrap_or(0.0);
                debug_assert!(
                    rotation_inc.is_finite() && (0.0..=1.0).contains(&rotation_inc),
                    "rotation_inc must be in [0,1]"
                );
                // A_ij(y) = (b_F/d_j) b_Hj - (b_F/d_j) (n_Hj·U_i^T) y
                let bf = hs[f].c;
                let a_vec = ut_i.transpose() * hs[h_idx_j].n;
                let scale = bf / d_j;
                let action_inc = Aff1 {
                    a: -a_vec * scale,
                    b: scale * hs[h_idx_j].c,
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

fn chart_is_lagrangian(chart_u: &Matrix2x4<f64>, j: &Matrix4<f64>) -> bool {
    chart_signed_omega(chart_u, j).abs() < TIGHT_EPS
}

fn chart_signed_omega(chart_u: &Matrix2x4<f64>, j: &Matrix4<f64>) -> f64 {
    let u1 = Vector4::new(
        chart_u[(0, 0)],
        chart_u[(0, 1)],
        chart_u[(0, 2)],
        chart_u[(0, 3)],
    );
    let u2 = Vector4::new(
        chart_u[(1, 0)],
        chart_u[(1, 1)],
        chart_u[(1, 2)],
        chart_u[(1, 3)],
    );
    u1.dot(&(j * u2))
}

fn ridge_poly_from_vertices(chart_u: &Matrix2x4<f64>, verts: &[Vector4<f64>]) -> Option<Poly2> {
    if verts.len() < 2 {
        return None;
    }
    let mut pts = Vec::with_capacity(verts.len());
    for v in verts {
        let y = chart_u * *v;
        let pt = Vector2::new(y[(0, 0)], y[(1, 0)]);
        pts.push(pt);
    }
    from_points_convex_hull_strict(&pts)
}
