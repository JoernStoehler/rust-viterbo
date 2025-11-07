//! Faces (1/2/3) and enumeration from H-representation.

use std::collections::{BTreeSet, HashMap};

use nalgebra::Vector4;

use super::cfg::{FEAS_EPS, TIGHT_EPS};
use super::convert::h_to_vertices;
use super::types::Hs4;
use super::util::{combinations, dedup_points_in_place};

/// Facet structure (3-face) from H-rep: defined by one saturated inequality.
#[derive(Clone, Debug)]
pub struct Face3 {
    pub facet_index: usize,
    pub vertices: Vec<Vector4<f64>>,
}

/// 2-face defined by two saturated inequalities (i,j) and its vertices.
#[derive(Clone, Debug)]
pub struct Face2 {
    pub facets: (usize, usize),
    pub vertices: Vec<Vector4<f64>>,
}

/// 1-face: edge defined by triple of saturated inequalities (i,j,k).
#[derive(Clone, Debug)]
pub struct Face1 {
    pub facets: (usize, usize, usize),
    pub vertices: Vec<Vector4<f64>>, // typically 2 vertices
}

/// Enumerate 0/1/2/3-faces from H-rep via vertex saturation.
///
/// Algorithm
/// - Convert H→V (if needed).
/// - For each vertex, record indices of near-tight inequalities.
/// - Group vertices by 1/2/3 saturated facets to get edges, 2-faces, facets.
///   Dedups are applied to handle degeneracy.
pub fn enumerate_faces_from_h(
    hs: &[Hs4],
) -> (Vec<Vector4<f64>>, Vec<Face1>, Vec<Face2>, Vec<Face3>) {
    let verts = h_to_vertices(hs);
    // For each vertex, collect which inequalities are (nearly) tight.
    let mut tight: Vec<BTreeSet<usize>> = Vec::with_capacity(verts.len());
    for (vi, &v) in verts.iter().enumerate() {
        let mut set = BTreeSet::new();
        for (i, h) in hs.iter().enumerate() {
            if (h.n.dot(&v) - h.c).abs() <= TIGHT_EPS {
                set.insert(i);
            }
        }
        tight.push(set);
        debug_assert!(tight[vi].len() >= 4 || hs.len() < 4);
    }
    // Facets: collect vertices by each inequality index.
    let mut facets: Vec<Face3> = Vec::new();
    for i in 0..hs.len() {
        let mut fverts = Vec::new();
        for (vi, v) in verts.iter().enumerate() {
            if tight[vi].contains(&i) {
                fverts.push(*v);
            }
        }
        if fverts.len() >= 3 {
            dedup_points_in_place(&mut fverts, FEAS_EPS);
            facets.push(Face3 {
                facet_index: i,
                vertices: fverts,
            });
        }
    }
    // 2-faces: pairs of facets
    let mut faces2_map: HashMap<(usize, usize), Vec<Vector4<f64>>> = HashMap::new();
    for (vi, v) in verts.iter().enumerate() {
        let idxs: Vec<usize> = tight[vi].iter().cloned().collect();
        for ij in combinations(&idxs, 2) {
            let key = (ij[0], ij[1]);
            faces2_map.entry(key).or_default().push(*v);
        }
    }
    let mut faces2: Vec<Face2> = faces2_map
        .into_iter()
        .filter_map(|((i, j), mut vs)| {
            dedup_points_in_place(&mut vs, FEAS_EPS);
            if vs.len() >= 2 {
                Some(Face2 {
                    facets: (i, j),
                    vertices: vs,
                })
            } else {
                None
            }
        })
        .collect();
    // 1-faces: triples
    let mut faces1_map: HashMap<(usize, usize, usize), Vec<Vector4<f64>>> = HashMap::new();
    for (vi, v) in verts.iter().enumerate() {
        let idxs: Vec<usize> = tight[vi].iter().cloned().collect();
        for ijk in combinations(&idxs, 3) {
            let key = (ijk[0], ijk[1], ijk[2]);
            faces1_map.entry(key).or_default().push(*v);
        }
    }
    let mut faces1: Vec<Face1> = faces1_map
        .into_iter()
        .filter_map(|((i, j, k), mut vs)| {
            dedup_points_in_place(&mut vs, FEAS_EPS);
            if vs.len() >= 2 {
                Some(Face1 {
                    facets: (i, j, k),
                    vertices: vs,
                })
            } else {
                None
            }
        })
        .collect();

    // Optional: dedup faces with identical vertex sets (can arise in degenerate cases).
    dedup_faces1(&mut faces1);
    dedup_faces2(&mut faces2);

    (verts, faces1, faces2, facets)
}

fn dedup_faces1(faces: &mut Vec<Face1>) {
    for f in faces.iter_mut() {
        dedup_points_in_place(&mut f.vertices, FEAS_EPS);
    }
    faces.dedup_by(|a, b| {
        if a.facets != b.facets {
            return false;
        }
        if a.vertices.len() != b.vertices.len() {
            return false;
        }
        a.vertices
            .iter()
            .zip(&b.vertices)
            .all(|(x, y)| (*x - *y).norm() < FEAS_EPS)
    });
}

fn dedup_faces2(faces: &mut Vec<Face2>) {
    for f in faces.iter_mut() {
        dedup_points_in_place(&mut f.vertices, FEAS_EPS);
    }
    faces.dedup_by(|a, b| {
        if a.facets != b.facets {
            return false;
        }
        if a.vertices.len() != b.vertices.len() {
            return false;
        }
        a.vertices
            .iter()
            .zip(&b.vertices)
            .all(|(x, y)| (*x - *y).norm() < FEAS_EPS)
    });
}

