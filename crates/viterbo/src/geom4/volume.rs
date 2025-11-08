//! 4D volume via facet fans anchored at an interior point.
//!
//! Why this module exists
//! - We need a deterministic, high-performance way to measure volumes of
//!   convex 4-polytopes described by lazy H/V caches (`Poly4`).
//! - The implementation triangulates each 3-face (facet) using its incident
//!   2-faces, cones those tetrahedra to an interior point, and sums the
//!   resulting 4-simplices. This avoids external deps and stays within the
//!   explicit enumeration style mandated by the thesis.
//!
//! References
//! - Docs: docs/src/thesis/geom4d_volume.md
//! - Ticket: 2224b2c6-4a0c-468d-a7a1-493eb2ee5ddd

use std::collections::HashMap;
use std::fmt;

use nalgebra::{Matrix3, Vector4};

use super::cfg::FEAS_EPS;
use super::faces::{enumerate_faces_from_h, Face2};
use super::types::{Hs4, Poly4};

// Clippy-friendly aliases for map shapes used during facet accumulation.
type Face2Key = usize;
type OrderedFace2 = (Vec<Vector4<f64>>, (usize, usize));
type Face2Lookup = HashMap<Face2Key, Vec<OrderedFace2>>;

/// Errors surfaced by the volume algorithm.
#[derive(Debug)]
pub enum VolumeError {
    /// Not enough half-spaces to enclose a H-rep polytope.
    NeedHalfspaces,
    /// Enumeration could not recover enough vertices (degenerate input).
    NeedVertices,
    /// A 2-face could not be ordered into a polygon.
    DegenerateFace2 { facets: (usize, usize) },
    /// A facet is missing incident 2-faces or has < 4 distinct vertices.
    DegenerateFacet { facet: usize },
}

impl fmt::Display for VolumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VolumeError::NeedHalfspaces => write!(f, "polytope has no half-spaces (empty volume)"),
            VolumeError::NeedVertices => {
                write!(f, "polytope has insufficient vertices for a 4D volume")
            }
            VolumeError::DegenerateFace2 { facets } => write!(
                f,
                "2-face defined by facets {:?} is degenerate (needs ≥3 unique vertices)",
                facets
            ),
            VolumeError::DegenerateFacet { facet } => write!(
                f,
                "facet {} is degenerate (needs ≥4 vertices and at least one incident 2-face)",
                facet
            ),
        }
    }
}

/// Compute the 4D volume using whatever representation `poly` already holds.
pub fn volume4(poly: &mut Poly4) -> Result<f64, VolumeError> {
    if poly.h.is_empty() {
        if poly.v.is_empty() {
            return Err(VolumeError::NeedHalfspaces);
        }
        poly.ensure_halfspaces_from_v();
    }
    if poly.h.is_empty() {
        return Err(VolumeError::NeedHalfspaces);
    }
    volume_from_halfspaces(&poly.h)
}

/// Compute the 4D volume directly from an H-representation.
pub fn volume_from_halfspaces(hs: &[Hs4]) -> Result<f64, VolumeError> {
    if hs.len() < 5 {
        return Err(VolumeError::NeedHalfspaces);
    }
    let (vertices, _edges, faces2, faces3) = enumerate_faces_from_h(hs);
    if vertices.len() < 5 {
        return Err(VolumeError::NeedVertices);
    }
    let center = centroid(&vertices);
    let face2_lookup = build_face2_lookup(&faces2)?;

    let mut total = 0.0;
    for facet in &faces3 {
        let ordered_faces =
            face2_lookup
                .get(&facet.facet_index)
                .ok_or(VolumeError::DegenerateFacet {
                    facet: facet.facet_index,
                })?;
        if facet.vertices.len() < 4 {
            return Err(VolumeError::DegenerateFacet {
                facet: facet.facet_index,
            });
        }
        let facet_center = centroid(&facet.vertices);
        let mut facet_volume = 0.0;
        for (polygon, facets) in ordered_faces {
            if polygon.len() < 3 {
                return Err(VolumeError::DegenerateFace2 { facets: *facets });
            }
            let anchor = polygon[0];
            for idx in 1..polygon.len() - 1 {
                let v1 = polygon[idx];
                let v2 = polygon[idx + 1];
                facet_volume += tetra_volume(facet_center, anchor, v1, v2);
            }
        }
        let hs = hs
            .get(facet.facet_index)
            .ok_or(VolumeError::DegenerateFacet {
                facet: facet.facet_index,
            })?;
        let norm = hs.n.norm();
        if norm <= FEAS_EPS {
            return Err(VolumeError::DegenerateFacet {
                facet: facet.facet_index,
            });
        }
        let height = (hs.c - hs.n.dot(&center)) / norm;
        if height < -FEAS_EPS {
            return Err(VolumeError::DegenerateFacet {
                facet: facet.facet_index,
            });
        }
        total += facet_volume * height.max(0.0) / 4.0;
    }

    Ok(total)
}

fn build_face2_lookup(faces: &[Face2]) -> Result<Face2Lookup, VolumeError> {
    let mut map: Face2Lookup = HashMap::new();
    for face in faces {
        let ordered = order_face2_vertices(&face.vertices).ok_or(VolumeError::DegenerateFace2 {
            facets: face.facets,
        })?;
        map.entry(face.facets.0)
            .or_default()
            .push((ordered.clone(), face.facets));
        map.entry(face.facets.1)
            .or_default()
            .push((ordered, face.facets));
    }
    Ok(map)
}

fn order_face2_vertices(points: &[Vector4<f64>]) -> Option<Vec<Vector4<f64>>> {
    if points.len() < 3 {
        return None;
    }
    let basis = plane_basis(points)?;
    let centroid = centroid(points);
    let mut items = Vec::with_capacity(points.len());
    for &p in points {
        let rel = p - centroid;
        let x = basis[0].dot(&rel);
        let y = basis[1].dot(&rel);
        let angle = y.atan2(x);
        if angle.is_nan() {
            return None;
        }
        items.push((angle, p));
    }
    items.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    Some(items.into_iter().map(|(_, p)| p).collect())
}

fn plane_basis(points: &[Vector4<f64>]) -> Option<[Vector4<f64>; 2]> {
    if points.len() < 3 {
        return None;
    }
    for i in 0..points.len() {
        for j in i + 1..points.len() {
            let mut v1 = points[j] - points[i];
            let norm1 = v1.norm();
            if norm1 <= FEAS_EPS {
                continue;
            }
            v1 /= norm1;
            for k in 0..points.len() {
                if k == i || k == j {
                    continue;
                }
                let mut v2 = points[k] - points[i];
                v2 -= v1 * v1.dot(&v2);
                let norm2 = v2.norm();
                if norm2 <= FEAS_EPS {
                    continue;
                }
                return Some([v1, v2 / norm2]);
            }
        }
    }
    None
}

fn centroid(points: &[Vector4<f64>]) -> Vector4<f64> {
    let mut acc = Vector4::zeros();
    for &p in points {
        acc += p;
    }
    acc / (points.len() as f64)
}

fn tetra_volume(a: Vector4<f64>, b: Vector4<f64>, c: Vector4<f64>, d: Vector4<f64>) -> f64 {
    let u1 = b - a;
    let u2 = c - a;
    let u3 = d - a;
    let gram = Matrix3::new(
        u1.dot(&u1),
        u1.dot(&u2),
        u1.dot(&u3),
        u2.dot(&u1),
        u2.dot(&u2),
        u2.dot(&u3),
        u3.dot(&u1),
        u3.dot(&u2),
        u3.dot(&u3),
    );
    let det = gram.determinant();
    if det <= 0.0 {
        return 0.0;
    }
    det.sqrt() / 6.0
}

#[cfg(test)]
mod tests {
    use super::{centroid, order_face2_vertices, tetra_volume, volume4, VolumeError};
    use nalgebra::{Matrix4, Vector4};

    use crate::geom4::types::{Hs4, Poly4};

    fn hypercube_poly(side: f64) -> Poly4 {
        let mut hs = Vec::new();
        for axis in 0..4 {
            let mut pos = Vector4::zeros();
            pos[axis] = 1.0;
            hs.push(Hs4::new(pos, side));
            let mut neg = Vector4::zeros();
            neg[axis] = -1.0;
            hs.push(Hs4::new(neg, side));
        }
        Poly4::from_h(hs)
    }

    #[test]
    fn tetra_volume_matches_formula() {
        let a = Vector4::new(0.0, 0.0, 0.0, 0.0);
        let b = Vector4::new(1.0, 0.0, 0.0, 0.0);
        let c = Vector4::new(0.0, 1.0, 0.0, 0.0);
        let d = Vector4::new(0.0, 0.0, 1.0, 0.0);
        let vol = tetra_volume(a, b, c, d);
        assert!((vol - (1.0 / 6.0)).abs() < 1e-12);
    }

    #[test]
    fn volume_hypercube() {
        let mut poly = hypercube_poly(1.0);
        let vol = volume4(&mut poly).unwrap();
        assert!((vol - 16.0).abs() < 1e-9, "computed volume {}", vol);
    }

    #[test]
    fn volume_simplex_matches_known_value() {
        let mut hs = Vec::new();
        for axis in 0..4 {
            let mut n = Vector4::zeros();
            n[axis] = -1.0;
            hs.push(Hs4::new(n, 0.0));
        }
        let n_sum = Vector4::new(1.0, 1.0, 1.0, 1.0);
        hs.push(Hs4::new(n_sum, 1.0));
        let mut poly = Poly4::from_h(hs);
        let vol = volume4(&mut poly).unwrap();
        assert!((vol - (1.0 / 24.0)).abs() < 1e-9, "computed volume {}", vol);
    }

    #[test]
    fn volume_invariant_under_det_one_affine_maps() {
        let mut poly = hypercube_poly(1.2);
        let base = volume4(&mut poly).unwrap();
        let m = Matrix4::<f64>::new(
            1.0, 0.1, 0.0, 0.0, 0.0, 1.0, 0.2, 0.0, 0.0, 0.0, 1.0, 0.3, 0.0, 0.0, 0.0, 1.0,
        );
        let det: f64 = m.determinant();
        assert!((det - 1.0).abs() < 1e-12);
        let t = Vector4::new(0.3, -0.2, 0.1, 0.4);
        let pushed = poly.push_forward(m, t).unwrap();
        let mut pushed = pushed;
        let vol = volume4(&mut pushed).unwrap();
        assert!((vol - base).abs() < 1e-8);
    }

    #[test]
    fn insufficient_halfspaces_is_error() {
        let mut poly = Poly4::default();
        assert!(matches!(
            volume4(&mut poly),
            Err(VolumeError::NeedHalfspaces)
        ));
    }

    #[test]
    fn plane_basis_orders_face2() {
        let pts = vec![
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(1.0, 1.0, 0.0, 0.0),
            Vector4::new(1.0, 1.0, 1.0, 0.0),
        ];
        let ordered = order_face2_vertices(&pts).unwrap();
        assert_eq!(ordered.len(), 3);
    }

    #[test]
    fn centroid_handles_many_points() {
        let pts = vec![
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        let c = centroid(&pts);
        assert!((c[0] - 0.25).abs() < 1e-12);
    }
}
