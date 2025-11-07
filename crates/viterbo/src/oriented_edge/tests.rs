//! Minimal smoke tests for the oriented-edge module.
//! These mirror the original in-file tests and validate core behavior.

use super::*;
use crate::geom4::{Hs4, Poly4};
use crate::{Aff1, GeomCfg};
use nalgebra::{Matrix2, Matrix2x4, Matrix4x2, Vector2, Vector4};

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
