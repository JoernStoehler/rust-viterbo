//! Core 4D types: half-spaces and polytopes with lazy H/V caches.

use nalgebra::{Matrix4, Vector4};

use super::cfg::FEAS_EPS;
use super::convert::{h_to_vertices, v_to_halfspaces};

/// Closed half-space `n · x <= c` in R^4.
///
/// Invariants:
/// - `n` is not normalized; `c` is any finite real.
/// - Membership uses `<= c + FEAS_EPS`.
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
        self.n.dot(&p) <= self.c + FEAS_EPS
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
        Some(self.h.iter().all(|h| h.c >= -FEAS_EPS))
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

