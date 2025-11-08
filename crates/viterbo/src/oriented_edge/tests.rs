//! Minimal smoke tests for the oriented-edge module.
//! These mirror the original in-file tests and validate core behavior.

use super::*;
use crate::geom4::{reeb_on_facets, Hs4, Poly4};
use crate::{Aff1, GeomCfg};
use nalgebra::{matrix, Matrix2, Matrix2x4, Matrix4x2, Vector2, Vector4};

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
    let mut p4 = Poly4::from_h(cube4_hs(1.0));
    let g = build_graph(&mut p4, GeomCfg::default());
    assert!(!g.ridges.is_empty());
    assert!(!g.edges.is_empty());
    // at least one edge has non-empty dom and img
    let has_valid = g.edges.iter().any(|e| {
        !e.dom_in.halfspace_intersection_eps(1e-9).is_empty()
            && !e.img_out.halfspace_intersection_eps(1e-9).is_empty()
    });
    assert!(has_valid);
}

#[test]
fn graph_ridge_facet_consistency_and_sorted_adjacency() {
    let mut p4 = Poly4::from_h(cube4_hs(1.0));
    let g = build_graph(&mut p4, GeomCfg::default());
    // Every edge's facet appears on both endpoints.
    for e in &g.edges {
        let ri = &g.ridges[e.from.0];
        let rj = &g.ridges[e.to.0];
        let f = e.facet;
        let has_i = ri.facets.0 == f || ri.facets.1 == f;
        let has_j = rj.facets.0 == f || rj.facets.1 == f;
        assert!(has_i && has_j, "facet label must be shared by both ridges");
    }
    // Adjacency lists are sorted by lb_action (ascending).
    for out in &g.adj {
        let mut last = f64::NEG_INFINITY;
        for &eidx in out {
            let v = g.edges[eidx].lb_action;
            assert!(
                v >= last - 1e-12,
                "adjacency must be non-decreasing by lb_action"
            );
            last = v;
        }
    }
}

#[test]
fn tau_domain_basic_properties_on_cube() {
    // For each edge, for each vertex of dom_in (when bounded), check:
    //   - forward denominator d_j > eps
    //   - for all forward‑hitting k in the same facet, τ_j <= τ_k holds.
    let cfg = GeomCfg::default();
    let mut p4 = Poly4::from_h(cube4_hs(1.0));
    let g = build_graph(&mut p4, cfg);
    let hs = &p4.h;
    let v_f = reeb_on_facets(hs);
    // Build helper: for each facet, list ridges in that facet.
    let mut by_facet: Vec<Vec<usize>> = vec![Vec::new(); g.num_facets];
    for (rid, r) in g.ridges.iter().enumerate() {
        by_facet[r.facets.0 .0].push(rid);
        by_facet[r.facets.1 .0].push(rid);
    }
    let other_facet = |r: &Ridge, f: usize| -> usize {
        let (a, b) = (r.facets.0 .0, r.facets.1 .0);
        if a == f {
            b
        } else {
            a
        }
    };
    // Check a subset to keep the test fast.
    let sample_edges = g.edges.iter().take(64);
    for e in sample_edges {
        let f = e.facet.0;
        let ri = &g.ridges[e.from.0];
        let rj = &g.ridges[e.to.0];
        let h_idx_j = other_facet(rj, f);
        let v = v_f[f];
        let d_j = hs[h_idx_j].n.dot(&v);
        assert!(
            d_j > cfg.eps_tau,
            "forward denominator must be positive for admissible j"
        );
        // Evaluate on vertices of dom_in when bounded.
        if let crate::geom2::ordered::HalfspaceIntersection::Bounded(verts) =
            e.dom_in.halfspace_intersection()
        {
            for y in verts {
                // x = U_i^T y in R^4
                let x4 = ri.chart_ut * y;
                // τ_j(y) numerators
                let num_j = hs[h_idx_j].c - hs[h_idx_j].n.dot(&x4);
                // For every k in the facet with d_k > 0, check inequality.
                for &rk in &by_facet[f] {
                    let r_k = &g.ridges[rk];
                    let h_idx_k = other_facet(r_k, f);
                    if h_idx_k == h_idx_j {
                        continue;
                    }
                    let d_k = hs[h_idx_k].n.dot(&v);
                    if d_k <= cfg.eps_tau {
                        continue;
                    }
                    let num_k = hs[h_idx_k].c - hs[h_idx_k].n.dot(&x4);
                    // inequality: d_k * num_j <= d_j * num_k (allow tiny slack)
                    let lhs = d_k * num_j;
                    let rhs = d_j * num_k + 1e-9;
                    assert!(
                        lhs <= rhs,
                        "τ-inequality violated at a dom vertex (lhs={lhs}, rhs={rhs})"
                    );
                }
            }
        }
    }
}

#[test]
fn smoke_dfs_trivial_cycle_manual_identity() {
    // Two ridges with identity mappings back and forth.
    let poly_unit = {
        let mut p = crate::geom2::Poly2::default();
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(1.0, 0.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(-1.0, 0.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(0.0, 1.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(0.0, -1.0), 1.0));
        p
    };
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
    let id = Affine2 {
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
fn cycle_closure_unique_fixed_point_on_tiny_graph() {
    // Two‑node cycle with contraction composing to ψ(z) = 0.5 z + t.
    // Unique fixed point z* = 2 t inside the unit box; action is zero.
    let poly_unit = {
        let mut p = crate::geom2::Poly2::default();
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(1.0, 0.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(-1.0, 0.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(0.0, 1.0), 1.0));
        p.insert_halfspace(crate::geom2::Hs2::new(Vector2::new(0.0, -1.0), 1.0));
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
            facets: (FacetId(2), FacetId(3)),
            poly: poly_unit.clone(),
            chart_u: u,
            chart_ut: ut,
        },
    ];
    // Compose M2*M1 = 0.5 I, with translations summing to t
    // Choose M1=M2= sqrt(0.5) I for simplicity.
    let s = (0.5f64).sqrt();
    let m_half = matrix![s, 0.0; 0.0, s];
    // Let total translation be t = [0.2, -0.1]; fixed point z* = 2t.
    let t_total = Vector2::new(0.2, -0.1);
    // Split translation consistently: ψ = M2(M1 z + t1) + t2 = (M2*M1) z + (M2 t1 + t2)
    // Choose t1 = t_total * 0.0, t2 = t_total (any split works as long as M2 t1 + t2 = t_total).
    let e01 = EdgeData {
        from: RidgeId(0),
        to: RidgeId(1),
        facet: FacetId(10),
        dom_in: poly_unit.clone(),
        img_out: poly_unit.clone(),
        map_ij: Affine2 {
            m: m_half,
            t: Vector2::new(0.0, 0.0),
        },
        action_inc: Aff1 {
            a: Vector2::new(0.0, 0.0),
            b: 0.0,
        },
        rotation_inc: 0.0,
        lb_action: 0.0,
    };
    let e10 = EdgeData {
        from: RidgeId(1),
        to: RidgeId(0),
        facet: FacetId(11),
        dom_in: poly_unit.clone(),
        img_out: poly_unit.clone(),
        map_ij: Affine2 {
            m: m_half,
            t: t_total,
        },
        action_inc: Aff1 {
            a: Vector2::new(0.0, 0.0),
            b: 0.0,
        },
        rotation_inc: 0.0,
        lb_action: 0.0,
    };
    let g = Graph {
        ridges,
        edges: vec![e01, e10],
        adj: vec![vec![0], vec![1]],
        num_facets: 12,
    };
    let res = dfs_solve_with_fp(&g, GeomCfg::default(), SearchCfg::default());
    assert!(res.is_some(), "unique fixed point must exist");
    let (best, cyc, z) = res.unwrap();
    assert!(best.abs() < 1e-12);
    assert_eq!(cyc.len(), 2);
    let z_star = t_total * 2.0;
    assert!(
        (z - z_star).norm() < 1e-9,
        "returned fixed point must match analytic z*"
    );
}

// Golden tests and invariance properties.
fn product_of_two_squares(a: f64, b: f64) -> crate::geom4::Poly4 {
    use crate::geom4::Hs4;
    use nalgebra::Vector4;
    let mut hs = Vec::new();
    // K in (x1,x2): |x1|<=a, |x2|<=a
    hs.push(Hs4::new(Vector4::new(1.0, 0.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(-1.0, 0.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 1.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, -1.0, 0.0, 0.0), a));
    // L in (y1,y2): |y1|<=b, |y2|<=b
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 1.0, 0.0), b));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, -1.0, 0.0), b));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 0.0, 1.0), b));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 0.0, -1.0), b));
    crate::geom4::Poly4::from_h(hs)
}

#[test]
fn golden_capacity_product_of_squares_matches_min_area() {
    // In R^2, normalized capacities coincide with area (Siburg 1993),
    // hence for K×L ⊂ R^2×R^2, c_EHZ(K×L) = min(area(K), area(L)).
    // Use K=[-1,1]^2 (area 4), L=[-2,2]^2 (area 16) → expect capacity 4.
    use crate::oriented_edge::{build_graph, dfs_solve, SearchCfg};
    let mut p4 = product_of_two_squares(1.0, 2.0);
    assert!(p4.check_canonical().is_ok());
    let g = build_graph(&mut p4, GeomCfg::default());
    let (best, _cycle) =
        dfs_solve(&g, GeomCfg::default(), SearchCfg::default()).expect("capacity exists");
    let expected = 4.0;
    assert!(
        (best - expected).abs() <= 5e-6,
        "capacity {best} vs expected {expected}"
    );
    // Volume (4D) is product of areas: 4 * 16 = 64. Systolic ratio vol / c^2 = 64 / 16 = 4.
    let volume = 64.0;
    let systolic = volume / (best * best);
    assert!(
        (systolic - 4.0).abs() <= 1e-6,
        "systolic ratio {systolic} vs 4"
    );
}

#[test]
fn golden_capacity_hypercube_minus1_1_pow4_is_4() {
    // Hypercube [-1,1]^4 = product of two unit squares → capacity 4 exactly.
    use crate::oriented_edge::{build_graph, dfs_solve, SearchCfg};
    let mut p4 = product_of_two_squares(1.0, 1.0);
    assert!(p4.check_canonical().is_ok());
    let g = build_graph(&mut p4, GeomCfg::default());
    let (best, _cycle) =
        dfs_solve(&g, GeomCfg::default(), SearchCfg::default()).expect("capacity exists");
    assert!((best - 4.0).abs() <= 5e-6, "capacity {best} vs 4");
}

#[test]
fn invariance_under_block_rotation_symplectomorphism() {
    // Capacity must be invariant under linear symplectomorphisms.
    // Use the product-of-squares example and a block rotation M=diag(R,R) (R∈SO(2)).
    use crate::geom4::is_symplectic;
    use crate::oriented_edge::{build_graph, dfs_solve, SearchCfg};
    use nalgebra::{Matrix2, Matrix4, Vector4};
    let mut base = product_of_two_squares(1.0, 2.0);
    assert!(base.check_canonical().is_ok());
    let gb = build_graph(&mut base, GeomCfg::default());
    let (c0, _cyc0) =
        dfs_solve(&gb, GeomCfg::default(), SearchCfg::default()).expect("capacity exists");
    // Block rotation
    let th = std::f64::consts::FRAC_PI_6; // 30°
    let r = Matrix2::new(th.cos(), -th.sin(), th.sin(), th.cos());
    let m = Matrix4::new(
        r[(0, 0)],
        r[(0, 1)],
        0.0,
        0.0,
        r[(1, 0)],
        r[(1, 1)],
        0.0,
        0.0,
        0.0,
        0.0,
        r[(0, 0)],
        r[(0, 1)],
        0.0,
        0.0,
        r[(1, 0)],
        r[(1, 1)],
    );
    assert!(is_symplectic(&m));
    let transformed = base.push_forward(m, Vector4::zeros()).expect("invertible");
    let mut p4_t = transformed.clone();
    assert!(p4_t.check_canonical().is_ok());
    let gt = build_graph(&mut p4_t, GeomCfg::default());
    let (c1, _cyc1) = dfs_solve(&gt, GeomCfg::default(), SearchCfg::default())
        .expect("capacity exists after transform");
    assert!(
        (c0 - c1).abs() <= 5e-6,
        "capacity must be invariant: {c0} vs {c1}"
    );
}

#[test]
fn cross_polytope_and_simplex_smoke_capacities() {
    use crate::geom4::special::cross_polytope_l1;
    use crate::oriented_edge::{build_graph, dfs_solve, SearchCfg};
    let cfg = GeomCfg::default();
    let scfg = SearchCfg {
        use_rotation_prune: false,
        rotation_budget: 2.0,
    };
    // Cross polytope (ℓ1 ball) radius 1.
    let mut cp = cross_polytope_l1(1.0);
    assert!(cp.check_canonical().is_ok());
    let g1 = build_graph(&mut cp, cfg);
    let (c_cp, _) = dfs_solve(&g1, cfg, SearchCfg::default()).expect("capacity cross polytope");
    assert!(c_cp.is_finite() && c_cp > 0.0);
    // Note: orthogonal_simplex is available in geom4::special; wiring it into oriented‑edge
    // is deferred until we settle H-rep conversion details for degenerate facets.
}
