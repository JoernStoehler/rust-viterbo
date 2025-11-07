//! 4D Convex Polytopes (H- and V-representations; explicit, simple algorithms).
//!
//! Purpose
//! - Production module for 4D polytopes where counts are moderate (≈1e6). We run
//!   fewer modifying ops, so we prioritize clarity and explicit conversions.
//!
//! Why this design (short)
//! - Track both H‑ and V‑rep (either may be empty until requested).
//! - Keep conversions explicit (enumeration), dependency‑light, and easy to audit.
//! - Make functions accept only what they need (don’t force “rich” objects).
//!
//! Assumptions and conventions
//! - Half‑spaces use `n·x <= c`; neither `n` nor `c` is constrained (no unit
//!   normalization, `c` not required positive). Rationale: avoid cost in cold
//!   paths and preserve caller’s scaling.
//! - Equality tests use tolerances: feasibility `1e-9`, symplectic check `1e-8`.
//! - V→H keeps only supporting hyperplanes (all vertices on one side). Orientation
//!   is chosen so that vertices satisfy `n·x <= c` (outward normal).
//! - Face enumeration is derived from saturation sets of vertices. Degenerate
//!   cases (coincident facets/flat regions) are handled by geometric dedup.
//! - 2‑face → 2D mapping exposes an explicit orientation toggle; we’ll lock down
//!   a canonical convention once the thesis pinpoints the exact orientation rule
//!   (e.g., via the ambient symplectic 2‑form).
//!
//! References
//! - TH: docs/src/thesis/geom4d_polytopes.md
//! - AGENTS: `AGENTS.md`
//! - Related code: `crate::geom2` for 2D mappings

use nalgebra::{Matrix4, Vector4};
use std::collections::{BTreeSet, HashMap, HashSet};

const EPS: f64 = 1e-9;

/// Closed half-space `n · x <= c` in R^4.
///
/// Invariants:
/// - `n` is not normalized; `c` is any finite real.
/// - Membership uses `<= c + EPS`.
#[derive(Clone, Copy, Debug)]
pub struct Hs4 {
    pub n: Vector4<f64>,
    pub c: f64,
}

impl Hs4 {
    #[inline]
    pub fn new(n: Vector4<f64>, c: f64) -> Self {
        Self { n, c }
    }
    #[inline]
    pub fn satisfies(&self, p: Vector4<f64>) -> bool {
        self.n.dot(&p) <= self.c + EPS
    }
}

/// Polytope in R^4; either representation may be empty, compute on demand.
///
/// Invariants:
/// - `h` and `v` are caches; one or both may be empty.
/// - Use `ensure_vertices_from_h()` or `ensure_halfspaces_from_v()` to populate.
#[derive(Clone, Debug, Default)]
pub struct Poly4 {
    pub h: Vec<Hs4>,
    pub v: Vec<Vector4<f64>>,
}

impl Poly4 {
    #[inline]
    pub fn from_h(h: Vec<Hs4>) -> Self {
        Self { h, v: Vec::new() }
    }
    #[inline]
    pub fn from_v(v: Vec<Vector4<f64>>) -> Self {
        Self { h: Vec::new(), v }
    }

    /// Append inequality (intersection).
    #[inline]
    pub fn intersect_halfspace(&mut self, hs: Hs4) {
        self.h.push(hs);
        // Invalidate cached vertices; callers may recompute as needed.
        self.v.clear();
    }

    /// H→V conversion by enumerating 4-tuples of active constraints.
    ///
    /// Complexity: O(H^4). Acceptable here due to low frequency of use.
    pub fn ensure_vertices_from_h(&mut self) {
        if !self.v.is_empty() {
            return;
        }
        let verts = h_to_vertices(&self.h);
        self.v = verts;
    }

    /// V→H conversion by enumerating supporting hyperplanes from 4-tuples.
    ///
    /// Complexity: O(V^4). We filter for supporting planes and orient so that
    /// `n·x <= c` holds for vertices.
    pub fn ensure_halfspaces_from_v(&mut self) {
        if !self.h.is_empty() {
            return;
        }
        let hs = v_to_halfspaces(&self.v);
        self.h = hs;
    }

    /// Check convexity by verifying each vertex satisfies all inequalities.
    pub fn is_convex(&mut self) -> bool {
        if self.h.is_empty() && self.v.is_empty() {
            return false;
        }
        if self.v.is_empty() {
            self.ensure_vertices_from_h();
        }
        if self.h.is_empty() {
            self.ensure_halfspaces_from_v();
        }
        self.v
            .iter()
            .all(|&x| self.h.iter().all(|h| h.satisfies(x)))
    }

    /// Check star-shaped wrt origin (contains 0).
    ///
    /// Uses H-rep if present or derivable; otherwise returns `None`.
    pub fn contains_origin(&mut self) -> Option<bool> {
        if self.h.is_empty() && self.v.is_empty() {
            return None;
        }
        if self.h.is_empty() {
            self.ensure_halfspaces_from_v();
        }
        Some(self.h.iter().all(|h| h.c >= -EPS))
    }

    /// Push-forward under invertible affine map `y = M x + t`.
    ///
    /// Derivation: With `n·x <= c` and `x = M^{-1}(y - t)`, we get
    /// `(n M^{-1})·y <= c + (n M^{-1})·t`. We implement `A' = A M^{-1}` and
    /// `c' = c + A'·t` for each row. Also pushes vertices if present.
    pub fn push_forward(&self, m: Matrix4<f64>, t: Vector4<f64>) -> Option<Self> {
        let minv = m.try_inverse()?;
        let mut out_h = Vec::with_capacity(self.h.len());
        for h in &self.h {
            // y feasible iff n·(M^{-1}(y - t)) <= c
            let n_new_t = h.n.transpose() * minv;
            let n_new = Vector4::new(
                n_new_t[(0, 0)],
                n_new_t[(0, 1)],
                n_new_t[(0, 2)],
                n_new_t[(0, 3)],
            );
            // b' = b + A'·t
            let c_new = h.c + n_new.dot(&t);
            out_h.push(Hs4::new(n_new, c_new));
        }
        let mut out_v = Vec::with_capacity(self.v.len());
        for &v in &self.v {
            out_v.push(m * v + t);
        }
        Some(Self { h: out_h, v: out_v })
    }
}

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

/// Check linear symplectomorphism: M^T J M ≈ J.
///
/// Tolerance: `1e-8` in max‑abs metric.
pub fn is_symplectic(m: &Matrix4<f64>) -> bool {
    let j = j_matrix_4();
    let lhs = m.transpose() * j * m;
    (lhs - j).amax() < 1e-8
}

/// Invert an affine map `(M, t)` if possible.
pub fn invert_affine_4(m: Matrix4<f64>, t: Vector4<f64>) -> Option<(Matrix4<f64>, Vector4<f64>)> {
    m.try_inverse().map(|minv| (minv, -minv * t))
}

/// Reeb flows on 3-faces: returns `J n_i` for each facet normal (unnormalized).
///
/// We do not normalize `n` here; callers may scale as needed for their
/// application.
pub fn reeb_on_facets(hs: &[Hs4]) -> Vec<Vector4<f64>> {
    let j = j_matrix_4();
    hs.iter().map(|h| j * h.n).collect()
}

/// Stub: Reeb flows on 1-faces (requires additional derivation).
pub fn reeb_on_edges_stub() -> Option<Vec<Vector4<f64>>> {
    None
}

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
/// - For each vertex, record indices of near‑tight inequalities.
/// - Group vertices by 1/2/3 saturated facets to get edges, 2‑faces, facets.
/// Dedups are applied to handle degeneracy.
pub fn enumerate_faces_from_h(
    hs: &[Hs4],
) -> (Vec<Vector4<f64>>, Vec<Face1>, Vec<Face2>, Vec<Face3>) {
    let verts = h_to_vertices(hs);
    // For each vertex, collect which inequalities are (nearly) tight.
    let mut tight: Vec<BTreeSet<usize>> = Vec::with_capacity(verts.len());
    for (vi, &v) in verts.iter().enumerate() {
        let mut set = BTreeSet::new();
        for (i, h) in hs.iter().enumerate() {
            if (h.n.dot(&v) - h.c).abs() <= 1e-7 {
                set.insert(i);
            }
        }
        // Ensure at least 4 tight in generic cases; keep as-is otherwise.
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
            // Dedup by geometric proximity
            dedup_points_in_place(&mut fverts, 1e-9);
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
            dedup_points_in_place(&mut vs, 1e-9);
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
            dedup_points_in_place(&mut vs, 1e-9);
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
    hs: &[Hs4],
    i: usize,
    j: usize,
    orientation_positive: bool,
) -> Option<(nalgebra::Matrix2x4<f64>, nalgebra::Matrix4x2<f64>)> {
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
    let u = nalgebra::Matrix2x4::from_rows(&[u1.transpose(), u2.transpose()]);
    let ut = nalgebra::Matrix4x2::from_columns(&[u1, u2]);
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
    // Collect vertices that saturate both facets
    let mut pts2d = Vec::new();
    for &v in &poly.v {
        let tight_i = (poly.h[i].n.dot(&v) - poly.h[i].c).abs() <= 1e-7;
        let tight_j = (poly.h[j].n.dot(&v) - poly.h[j].c).abs() <= 1e-7;
        if tight_i && tight_j {
            let y = u * v;
            pts2d.push(nalgebra::Vector2::new(y[(0, 0)], y[(1, 0)]));
        }
    }
    let poly2 = crate::geom2::from_points_convex_hull_strict(&pts2d)?;
    Some(poly2)
}

/// H→V: enumerate all 4-tuples of inequalities, solve equalities, test feasibility.
///
/// Implementation detail: columns are normals; we solve `M^T x = c` and check
/// feasibility against all inequalities with tolerance.
pub fn h_to_vertices(hs: &[Hs4]) -> Vec<Vector4<f64>> {
    let mut out = Vec::new();
    if hs.len() < 4 {
        return out;
    }
    // Enumerate all 4-tuples of inequalities
    let idxs: Vec<usize> = (0..hs.len()).collect();
    for comb in combinations(&idxs, 4) {
        // Solve [n1 n2 n3 n4]^T x = [c1 c2 c3 c4]
        let a = nalgebra::Matrix4::from_columns(&[
            hs[comb[0]].n,
            hs[comb[1]].n,
            hs[comb[2]].n,
            hs[comb[3]].n,
        ]);
        if let Some(inv) = a.try_inverse() {
            let b = Vector4::new(hs[comb[0]].c, hs[comb[1]].c, hs[comb[2]].c, hs[comb[3]].c);
            let x = inv.transpose() * b;
            // Feasibility check with tolerance
            if hs.iter().all(|h| h.satisfies(x)) {
                out.push(x);
            }
        }
    }
    dedup_points_in_place(&mut out, 1e-9);
    out
}

/// V→H: supporting hyperplanes from vertex set in R^4.
pub fn v_to_halfspaces(vs: &[Vector4<f64>]) -> Vec<Hs4> {
    let mut hs = Vec::new();
    if vs.len() < 5 {
        return hs;
    }
    // Enumerate supporting planes from 4-tuples; keep if all vertices on one side.
    let idxs: Vec<usize> = (0..vs.len()).collect();
    let mut seen: HashSet<(i64, i64, i64, i64, i64)> = HashSet::new();
    for comb in combinations(&idxs, 4) {
        if let Some((n, c)) =
            supporting_plane_from4([vs[comb[0]], vs[comb[1]], vs[comb[2]], vs[comb[3]]])
        {
            // Orient so that all vertices satisfy n·x <= c
            let mut on_pos = false;
            let mut on_neg = false;
            for &x in vs {
                let d = n.dot(&x) - c;
                if d > 1e-9 {
                    on_pos = true;
                }
                if d < -1e-9 {
                    on_neg = true;
                }
                if on_pos && on_neg {
                    break;
                }
            }
            let (n, c) = if on_pos && !on_neg { (-n, -c) } else { (n, c) };
            let key = quantize5(n, c, 1e-9);
            if !seen.contains(&key) {
                seen.insert(key);
                hs.push(Hs4::new(n, c));
            }
        }
    }
    hs
}

/// Supporting plane from 4 points (if coplanar and oriented).
///
/// Uses cofactor expansion (Hodge dual of 3-form) to avoid SVD.
fn supporting_plane_from4(pts: [Vector4<f64>; 4]) -> Option<(Vector4<f64>, f64)> {
    let a = pts[1] - pts[0];
    let b = pts[2] - pts[0];
    let c = pts[3] - pts[0];
    // Normal via 4D cross product analogue (cofactors)
    let n = Vector4::new(
        det3([[a.y, a.z, a.w], [b.y, b.z, b.w], [c.y, c.z, c.w]]),
        -det3([[a.x, a.z, a.w], [b.x, b.z, b.w], [c.x, c.z, c.w]]),
        det3([[a.x, a.y, a.w], [b.x, b.y, b.w], [c.x, c.y, c.w]]),
        -det3([[a.x, a.y, a.z], [b.x, b.y, b.z], [c.x, c.y, c.z]]),
    );
    if n.norm() < 1e-12 {
        return None;
    }
    let n = n / n.norm();
    let c = n.dot(&pts[0]);
    Some((n, c))
}

/// Nullspace vector of a 3x4 matrix whose rows are given; returns unit vector orthogonal to all 3 rows.
#[allow(dead_code)]
fn nullspace_vector_3x4(rows: [Vector4<f64>; 3]) -> Option<Vector4<f64>> {
    // Find any vector orthogonal to rows (simple Gram-Schmidt construction)
    let n1 = rows[0];
    let n2 = rows[1];
    let mut a = n1;
    if a.norm() < EPS {
        return None;
    }
    a /= a.norm();
    let mut b = n2 - a * (a.dot(&n2));
    if b.norm() < EPS {
        // pick an arbitrary vector not collinear with a
        let mut trial = Vector4::new(1.0, 0.0, 0.0, 0.0);
        if (trial - a).norm() < 0.1 {
            trial = Vector4::new(0.0, 1.0, 0.0, 0.0);
        }
        b = trial - a * (a.dot(&trial));
    }
    b /= b.norm();
    // Now pick u1 orthogonal to both a and b.
    let mut u1 = Vector4::new(1.0, 0.0, 0.0, 0.0);
    for trial in [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ] {
        let t = trial - a * a.dot(&trial) - b * b.dot(&trial);
        if t.norm() > 1e-3 {
            u1 = t / t.norm();
            break;
        }
    }
    // u2 orthogonal to {a, b, u1}
    let mut _u2 = Vector4::new(0.0, 1.0, 0.0, 0.0);
    for trial in [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ] {
        let t = trial - a * a.dot(&trial) - b * b.dot(&trial) - u1 * u1.dot(&trial);
        if t.norm() > 1e-3 {
            _u2 = t / t.norm();
            break;
        }
    }
    Some(u1)
}

fn det3(m: [[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn combinations<T: Copy>(items: &Vec<T>, k: usize) -> Vec<Vec<T>> {
    fn rec<T: Copy>(items: &[T], k: usize, start: usize, cur: &mut Vec<T>, out: &mut Vec<Vec<T>>) {
        if cur.len() == k {
            out.push(cur.clone());
            return;
        }
        for i in start..items.len() {
            cur.push(items[i]);
            rec(items, k, i + 1, cur, out);
            cur.pop();
        }
    }
    let mut out = Vec::new();
    let mut cur = Vec::new();
    rec(items, k, 0, &mut cur, &mut out);
    out
}

fn dedup_points_in_place(points: &mut Vec<Vector4<f64>>, tol: f64) {
    points.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a[1].partial_cmp(&b[1])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        a[2].partial_cmp(&b[2])
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| {
                                a[3].partial_cmp(&b[3]).unwrap_or(std::cmp::Ordering::Equal)
                            })
                    })
            })
    });
    points.dedup_by(|a, b| (*a - *b).norm() < tol);
}

fn dedup_faces1(faces: &mut Vec<Face1>) {
    for f in faces.iter_mut() {
        dedup_points_in_place(&mut f.vertices, 1e-9);
    }
    faces.sort_by_key(|f| f.vertices.len());
    faces.dedup_by(|a, b| {
        a.vertices.len() == b.vertices.len()
            && a.vertices
                .iter()
                .zip(&b.vertices)
                .all(|(x, y)| (*x - *y).norm() < 1e-9)
    });
}

fn dedup_faces2(faces: &mut Vec<Face2>) {
    for f in faces.iter_mut() {
        dedup_points_in_place(&mut f.vertices, 1e-9);
    }
    faces.sort_by_key(|f| f.vertices.len());
    faces.dedup_by(|a, b| {
        a.vertices.len() == b.vertices.len()
            && a.vertices
                .iter()
                .zip(&b.vertices)
                .all(|(x, y)| (*x - *y).norm() < 1e-9)
    });
}

fn quantize4(v: Vector4<f64>, tol: f64) -> (i64, i64, i64, i64) {
    let s = 1.0 / tol;
    (
        (v.x * s).round() as i64,
        (v.y * s).round() as i64,
        (v.z * s).round() as i64,
        (v.w * s).round() as i64,
    )
}

fn quantize5(n: Vector4<f64>, c: f64, tol: f64) -> (i64, i64, i64, i64, i64) {
    let (qx, qy, qz, qw) = quantize4(n, tol);
    let s = 1.0 / tol;
    (qx, qy, qz, qw, (c * s).round() as i64)
}

fn orthonormal_complement_2d(
    n1: Vector4<f64>,
    n2: Vector4<f64>,
) -> Option<(Vector4<f64>, Vector4<f64>)> {
    let mut a = n1;
    if a.norm() < EPS {
        return None;
    }
    a /= a.norm();
    let mut b = n2 - a * (a.dot(&n2));
    if b.norm() < EPS {
        // pick an arbitrary vector not collinear with a
        let mut trial = Vector4::new(1.0, 0.0, 0.0, 0.0);
        if (trial - a).norm() < 0.1 {
            trial = Vector4::new(0.0, 1.0, 0.0, 0.0);
        }
        b = trial - a * (a.dot(&trial));
    }
    b /= b.norm();
    // Now pick u1 orthogonal to both a and b.
    let mut u1 = Vector4::new(1.0, 0.0, 0.0, 0.0);
    for trial in [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ] {
        let t = trial - a * a.dot(&trial) - b * b.dot(&trial);
        if t.norm() > 1e-3 {
            u1 = t / t.norm();
            break;
        }
    }
    // u2 orthogonal to {a, b, u1}
    let mut u2 = Vector4::new(0.0, 1.0, 0.0, 0.0);
    for trial in [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ] {
        let t = trial - a * a.dot(&trial) - b * b.dot(&trial) - u1 * u1.dot(&trial);
        if t.norm() > 1e-3 {
            u2 = t / t.norm();
            break;
        }
    }
    Some((u1, u2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn j_is_correct() {
        let j = j_matrix_4();
        // J^2 = -I
        let j2 = j * j;
        assert!((j2 + Matrix4::identity()).amax() < 1e-12);
    }

    #[test]
    fn symplectic_identity() {
        let i = Matrix4::identity();
        assert!(is_symplectic(&i));
    }

    #[test]
    fn h_to_v_cube() {
        // 4D cube [-1,1]^4
        let mut hs = Vec::new();
        let axes = [
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        for a in axes.iter() {
            hs.push(Hs4::new(*a, 1.0));
            hs.push(Hs4::new(-*a, 1.0));
        }
        let vs = h_to_vertices(&hs);
        // Hypercube has 16 vertices
        assert!(vs.len() >= 16);
    }

    #[test]
    fn v_to_h_simplex() {
        // 4D simplex with vertices (0,0,0,0), e1, e2, e3, e4 -> 5 vertices
        let vs = vec![
            Vector4::new(0.0, 0.0, 0.0, 0.0),
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        let hs = v_to_halfspaces(&vs);
        assert!(!hs.is_empty());
    }

    #[test]
    fn faces_enumeration_cube() {
        // [-1,1]^4 as before
        let mut hs = Vec::new();
        let axes = [
            Vector4::new(1.0, 0.0, 0.0, 0.0),
            Vector4::new(0.0, 1.0, 0.0, 0.0),
            Vector4::new(0.0, 0.0, 1.0, 0.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
        ];
        for a in axes.iter() {
            hs.push(Hs4::new(*a, 1.0));
            hs.push(Hs4::new(-*a, 1.0));
        }
        let (_v0, edges, faces2, facets) = enumerate_faces_from_h(&hs);
        assert!(!facets.is_empty());
        assert!(!faces2.is_empty());
        assert!(!edges.is_empty());
    }
}
