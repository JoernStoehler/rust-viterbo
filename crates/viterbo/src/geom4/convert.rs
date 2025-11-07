//! H↔V conversions and supporting plane helpers.

use std::collections::HashSet;

use nalgebra::Vector4;

use super::cfg::FEAS_EPS;
use super::types::Hs4;
use super::util::{combinations, dedup_points_in_place, quantize5};

pub(crate) fn h_to_vertices(hs: &[Hs4]) -> Vec<Vector4<f64>> {
    let mut out = Vec::new();
    if hs.len() < 4 {
        return out;
    }
    // indices 0..H
    let idxs: Vec<usize> = (0..hs.len()).collect();
    // Enumerate 4-tuples and intersect their planes (if non-parallel and feasible).
    for comb in combinations(&idxs, 4) {
        let h1 = hs[comb[0]];
        let h2 = hs[comb[1]];
        let h3 = hs[comb[2]];
        let h4 = hs[comb[3]];
        let a = nalgebra::Matrix4::from_rows(&[
            h1.n.transpose(),
            h2.n.transpose(),
            h3.n.transpose(),
            h4.n.transpose(),
        ]);
        if let Some(inv) = a.try_inverse() {
            let b = nalgebra::Vector4::new(h1.c, h2.c, h3.c, h4.c);
            let x = inv * b;
            if hs.iter().all(|h| h.satisfies(x)) {
                out.push(x);
            }
        }
    }
    // Geometric dedup
    dedup_points_in_place(&mut out, FEAS_EPS);
    out
}

pub(crate) fn v_to_halfspaces(vs: &[Vector4<f64>]) -> Vec<Hs4> {
    let mut out = Vec::new();
    if vs.len() < 4 {
        return out;
    }
    // indices 0..V
    let idxs: Vec<usize> = (0..vs.len()).collect();
    // Enumerate 4-tuples of vertices → candidate planes.
    let mut seen = HashSet::new();
    for comb in combinations(&idxs, 4) {
        let pts = [vs[comb[0]], vs[comb[1]], vs[comb[2]], vs[comb[3]]];
        if let Some((n, c)) = supporting_plane_from4(pts) {
            // orient so that all points satisfy n·x <= c (outward normal)
            let mut side_ok = true;
            for &v in vs {
                if n.dot(&v) > c + FEAS_EPS {
                    side_ok = false;
                    break;
                }
            }
            if side_ok {
                // quantize to dedup numerically equal planes
                let key = quantize5(n, c, FEAS_EPS);
                if seen.insert(key) {
                    out.push(Hs4::new(n, c));
                }
            }
        }
    }
    out
}

fn supporting_plane_from4(pts: [Vector4<f64>; 4]) -> Option<(Vector4<f64>, f64)> {
    // Solve n·x = c for 4 points: [p1^T; p2^T; p3^T; p4^T] n = [c; c; c; c]
    // Subtract row p1 from others to get 3x4 linear system A n = 0; find a nonzero nullspace vector.
    let rows = [pts[1] - pts[0], pts[2] - pts[0], pts[3] - pts[0]];
    let n = nullspace_vector_3x4(rows)?;
    // Normalize and compute c = n·p1 with sign so that c>=0 (convention)
    let norm = n.norm();
    if norm <= 0.0 || !norm.is_finite() {
        return None;
    }
    let n = n / norm;
    let c = n.dot(&pts[0]).abs();
    Some((n, c))
}

fn nullspace_vector_3x4(rows: [Vector4<f64>; 3]) -> Option<Vector4<f64>> {
    // Find n ≠ 0 such that rows[i]·n = 0. Use 4x4 minors approach.
    // n_k = (-1)^k det(A_k), where A_k is the 3x3 minor removing column k.
    let a = [
        [rows[0][0], rows[0][1], rows[0][2], rows[0][3]],
        [rows[1][0], rows[1][1], rows[1][2], rows[1][3]],
        [rows[2][0], rows[2][1], rows[2][2], rows[2][3]],
    ];
    let n0 = det3([[a[0][1], a[0][2], a[0][3]], [a[1][1], a[1][2], a[1][3]], [a[2][1], a[2][2], a[2][3]]]);
    let n1 = -det3([[a[0][0], a[0][2], a[0][3]], [a[1][0], a[1][2], a[1][3]], [a[2][0], a[2][2], a[2][3]]]);
    let n2 = det3([[a[0][0], a[0][1], a[0][3]], [a[1][0], a[1][1], a[1][3]], [a[2][0], a[2][1], a[2][3]]]);
    let n3 = -det3([[a[0][0], a[0][1], a[0][2]], [a[1][0], a[1][1], a[1][2]], [a[2][0], a[2][1], a[2][2]]]);
    let n = Vector4::new(n0, n1, n2, n3);
    if !n.iter().all(|x| x.is_finite()) {
        return None;
    }
    Some(n)
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}
