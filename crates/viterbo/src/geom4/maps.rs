//! Symplectic matrices, Reeb directions, and 2D face chart maps.

use nalgebra::{Matrix2, Matrix2x4, Matrix4, Matrix4x2, Vector2, Vector4};
use super::cfg::{SYMPLECTIC_EPS, TIGHT_EPS};
use super::types::Poly4;

/// Return J = [[0, -I],[I, 0]] with 2x2 blocks.
///
/// Convention: coordinates ordered as (x1,x2,y1,y2) so that the standard
/// symplectic form is `ω = dx1∧dy1 + dx2∧dy2` and `J^2 = -I`.
#[inline]
pub fn j_matrix_4() -> Matrix4<f64> {
    Matrix4::new(
        0.0, 0.0, -1.0, 0.0, //
        0.0, 0.0, 0.0, -1.0, //
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0,
    )
}

/// Check linear symplectomorphism: M^T J M ≈ J (max‑abs metric).
pub fn is_symplectic(m: &Matrix4<f64>) -> bool {
    let j = j_matrix_4();
    let lhs = m.transpose() * j * m;
    (lhs - j).amax() < SYMPLECTIC_EPS
}

/// Invert an affine map `(M, t)` if possible.
pub fn invert_affine_4(m: Matrix4<f64>, t: Vector4<f64>) -> Option<(Matrix4<f64>, Vector4<f64>)> {
    m.try_inverse().map(|minv| (minv, -minv * t))
}

/// Reeb flows on 3-faces: returns `J n_i` for each facet normal (unnormalized).
///
/// We do not normalize `n` here; callers may scale as needed for their
/// application.
pub fn reeb_on_facets(hs: &[super::types::Hs4]) -> Vec<Vector4<f64>> {
    let j = j_matrix_4();
    hs.iter().map(|h| j * h.n).collect()
}

/// Stub: Reeb flows on 1-faces (requires additional derivation).
pub fn reeb_on_edges_stub() -> Option<Vec<Vector4<f64>>> {
    None
}

/// Sample a random linear symplectomorphism `M ∈ Sp(4, R)` using simple generators.
///
/// Construction (n=2):
/// - Draw `A ∈ GL(2, R)` with `det(A) > 0`.
/// - Draw symmetric `B, C`.
/// - Compose `M = D(A) · S(B) · T(C)` where
///   - `D(A) = [[A, 0], [0, A^{-T}]]`,
///   - `S(B) = [[I, B], [0, I]]` (B symmetric),
///   - `T(C) = [[I, 0], [C, I]]` (C symmetric).
#[allow(clippy::identity_op)]
pub fn random_symplectic_4(seed: u64) -> Matrix4<f64> {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    let mut rng = StdRng::seed_from_u64(seed);
    // Moderately conditioned A with det>0
    let a = loop {
        let m = Matrix2::new(
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
        );
        let det: f64 = m.determinant();
        if det.abs() > 0.2 && det.is_finite() {
            let m_ok = if det < 0.0 {
                Matrix2::new(-m[(0, 0)], -m[(0, 1)], m[(1, 0)], m[(1, 1)])
            } else {
                m
            };
            break m_ok;
        }
    };
    let a_inv_t = a.try_inverse().unwrap().transpose();
    let d = Matrix4::new(
        a[(0, 0)], a[(0, 1)], 0.0, 0.0, //
        a[(1, 0)], a[(1, 1)], 0.0, 0.0, //
        0.0, 0.0, a_inv_t[(0, 0)], a_inv_t[(0, 1)], //
        0.0, 0.0, a_inv_t[(1, 0)], a_inv_t[(1, 1)],
    );
    // Symmetric B, C with small magnitude
    let sym = |r: &mut StdRng, scale: f64| Matrix2::new(
        r.gen_range(-scale..=scale),
        r.gen_range(-scale..=scale),
        0.0,
        r.gen_range(-scale..=scale),
    );
    let mut b = sym(&mut rng, 0.5);
    b[(1, 0)] = b[(0, 1)];
    let mut c = sym(&mut rng, 0.5);
    c[(1, 0)] = c[(0, 1)];
    let s = Matrix4::new(
        1.0, 0.0, b[(0, 0)], b[(0, 1)], //
        0.0, 1.0, b[(1, 0)], b[(1, 1)], //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    );
    let t = Matrix4::new(
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        c[(0, 0)], c[(0, 1)], 1.0, 0.0, //
        c[(1, 0)], c[(1, 1)], 0.0, 1.0,
    );
    d * s * t
}

/// Build a 2D map for a 2-face given by two facet indices (i,j).
///
/// Returns (U, U^T) where U is 2x4 with orthonormal rows spanning the face
/// plane. The forward map is `y = U x`; inverse on the plane is `x = U^T y`.
///
/// Canonical orientation policy
/// - We enforce the unique “natural” orientation induced by the ambient
///   symplectic form: require ω0(u1, u2) = u1^T J u2 > 0. Near‑Lagrangian faces
///   (|ω0(u1,u2)| < TIGHT_EPS) are rejected.
pub fn oriented_orth_map_face2(
    hs: &[super::types::Hs4],
    i: usize,
    j: usize,
) -> Option<(Matrix2x4<f64>, Matrix4x2<f64>)> {
    if i >= hs.len() || j >= hs.len() || i == j {
        return None;
    }
    let n1 = hs[i].n.normalize();
    let n2 = hs[j].n.normalize();
    // Orthonormal basis of the 2D face plane = orthogonal complement of span{n1, n2}.
    let (u1, u2) = orthonormal_complement_2d(n1, n2)?;
    // Enforce canonical orientation:
    // 1) Prefer the “natural” ω0-induced orientation: require ω0(u1,u2)=u1^T J u2 > 0.
    // 2) If |ω0(u1,u2)| is ~0 (Lagrangian face), fall back to ambient R^4 orientation:
    //    require det([u1,u2,n1,n2]) > 0.
    let j = j_matrix_4();
    let omega = u1.dot(&(j * u2));
    let (u1, u2) = if omega.abs() >= TIGHT_EPS {
        if omega > 0.0 {
            (u1, u2)
        } else {
            (u1, -u2)
        }
    } else {
        // Build 4x4 matrix with columns [u1 u2 n1 n2] and check its determinant.
        let m = Matrix4::from_columns(&[u1, u2, n1, n2]);
        let det = m.determinant();
        if det.is_finite() && det.abs() >= TIGHT_EPS {
            if det > 0.0 {
                (u1, u2)
            } else {
                (u1, -u2)
            }
        } else {
            // Highly degenerate basis (shouldn't happen with distinct facets); keep as-is.
            (u1, u2)
        }
    };
    let u = Matrix2x4::from_rows(&[u1.transpose(), u2.transpose()]);
    let ut = Matrix4x2::from_columns(&[u1, u2]);
    Some((u, ut))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom4::Hs4;

    #[test]
    fn canonical_chart_has_positive_omega_on_generic_face() {
        // Build a small set of generic halfspaces; we only need two indices i!=j.
        let hs = vec![
            Hs4::new(Vector4::new(1.0, 1.0, 0.0, 0.0).normalize(), 1.0),
            Hs4::new(Vector4::new(0.0, 1.0, 1.0, 0.0).normalize(), 1.0),
            Hs4::new(Vector4::new(0.0, 0.0, 1.0, 1.0).normalize(), 1.0),
        ];
        // Pick the first two as a typical (generic) ridge pair.
        let (u, ut) = oriented_orth_map_face2(&hs, 0, 1).expect("chart");
        // Extract basis vectors from U^T columns.
        let u1 = ut.column(0).into_owned();
        let u2 = ut.column(1).into_owned();
        let j = j_matrix_4();
        let omega = u1.dot(&(j * u2));
        assert!(omega.abs() >= TIGHT_EPS, "face should be non-Lagrangian");
        assert!(
            omega > 0.0,
            "canonical chart must have omega(u1,u2)>0; got {omega}"
        );
        // Sanity: U is 2x4 with orthonormal rows (U U^T = I_2).
        let uu_t = u * ut;
        let id = nalgebra::Matrix2::<f64>::identity();
        assert!((uu_t - id).amax() < 1e-9);
    }
}

/// Build a 2D H-rep polytope for the 2-face (i,j) by projecting the face's
/// vertices with `y = U x` and taking their convex hull in 2D.
///
/// Returns `None` if too few projected vertices for a 2D hull.
pub fn face2_as_poly2_hrep(
    poly: &mut Poly4,
    i: usize,
    j: usize,
) -> Option<crate::geom2::Poly2> {
    let (u, _ut) = oriented_orth_map_face2(&poly.h, i, j)?;
    // Ensure vertices exist
    poly.ensure_vertices_from_h();
    if poly.v.len() < 2 {
        return None;
    }
    let mut pts2 = Vec::with_capacity(poly.v.len());
    for &x in &poly.v {
        let y = u * x;
        pts2.push(Vector2::new(y[0], y[1]));
    }
    crate::geom2::from_points_convex_hull_strict(&pts2)
}

fn orthonormal_complement_2d(
    n1: Vector4<f64>,
    n2: Vector4<f64>,
) -> Option<(Vector4<f64>, Vector4<f64>)> {
    // Find two orthonormal vectors spanning {n1,n2}^⊥ via Gram–Schmidt.
    let mut v = Vector4::new(1.0, 2.0, 3.0, 5.0);
    // project out components along n1 and n2
    for n in [n1, n2] {
        let alpha = v.dot(&n) / n.dot(&n);
        v -= n * alpha;
    }
    let u1 = v / v.norm();
    // pick another seed
    let mut w = Vector4::new(-2.0, 1.0, 0.5, -1.0);
    for n in [n1, n2] {
        let alpha = w.dot(&n) / n.dot(&n);
        w -= n * alpha;
    }
    // remove component along u1
    w -= u1 * w.dot(&u1);
    let u2 = w / w.norm();
    Some((u1, u2))
}
