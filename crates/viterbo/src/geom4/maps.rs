//! Symplectic matrices, Reeb directions, and 2D face chart maps.

use nalgebra::{Matrix2x4, Matrix4, Matrix4x2, Vector2, Vector4};

use super::cfg::SYMPLECTIC_EPS;
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

/// Build a 2D map for a 2-face given by two facet indices (i,j).
///
/// Returns (U, U^T) where U is 2x4 with orthonormal rows spanning the face
/// plane, oriented so that `orientation_positive == true` selects the sign.
/// The forward map is `y = U x`; inverse on the plane is `x = U^T y`.
///
/// Orientation policy
/// - This function only toggles the sign to give callers control. A canonical
///   orientation (e.g., compatible with the ambient symplectic 2‑form) can be
///   imposed later once the thesis fixes the convention.
pub fn oriented_orth_map_face2(
    hs: &[super::types::Hs4],
    i: usize,
    j: usize,
    orientation_positive: bool,
) -> Option<(Matrix2x4<f64>, Matrix4x2<f64>)> {
    if i >= hs.len() || j >= hs.len() || i == j {
        return None;
    }
    let n1 = hs[i].n.normalize();
    let n2 = hs[j].n.normalize();
    // Orthonormal basis of the 2D face plane = orthogonal complement of span{n1, n2}.
    let (u1, u2) = orthonormal_complement_2d(n1, n2)?;
    let (u1, u2) = if orientation_positive {
        (u1, u2)
    } else {
        (u1, -u2)
    };
    let u = Matrix2x4::from_rows(&[u1.transpose(), u2.transpose()]);
    let ut = Matrix4x2::from_columns(&[u1, u2]);
    Some((u, ut))
}

/// Build a 2D H-rep polytope for the 2-face (i,j) by projecting the face's
/// vertices with `y = U x` and taking their convex hull in 2D.
///
/// Returns `None` if too few projected vertices for a 2D hull.
pub fn face2_as_poly2_hrep(
    poly: &mut Poly4,
    i: usize,
    j: usize,
    orientation_positive: bool,
) -> Option<crate::geom2::Poly2> {
    let (u, _ut) = oriented_orth_map_face2(&poly.h, i, j, orientation_positive)?;
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
