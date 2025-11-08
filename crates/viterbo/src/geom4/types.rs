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
    /// Normalize to unit normal if possible: returns `(n/||n||, c/||n||)`.
    /// Returns `None` if the normal is near-zero.
    pub fn normalized(&self) -> Option<Self> {
        let norm = self.n.norm();
        if norm <= 1e-12 {
            None
        } else {
            Some(Self {
                n: self.n / norm,
                c: self.c / norm,
            })
        }
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

/// Canonicalize H-representation:
/// - normalize each half-space to unit normal,
/// - drop redundant/unsupported facets using vertex set (if bounded),
/// - preserve original relative order of remaining facets.
fn canonicalize_h_strict(hs: Vec<Hs4>) -> Vec<Hs4> {
    // Normalize and drop degenerate.
    let tmp: Vec<Hs4> = hs.iter().filter_map(|h| h.normalized()).collect();
    if tmp.is_empty() {
        return tmp;
    }
    // Redundancy pruning: keep only facets that are near-active on some vertex.
    // Compute vertices once from the current set (may be expensive but robust).
    let mut poly = Poly4 {
        h: tmp.clone(),
        v: Vec::new(),
    };
    poly.ensure_vertices_from_h();
    if poly.v.is_empty() {
        // Unbounded or degenerate: return normalized set as-is.
        return tmp;
    }
    let verts = poly.v.clone();
    let tight = super::cfg::TIGHT_EPS;
    let mut keep = vec![false; tmp.len()];
    for (i, h) in tmp.iter().enumerate() {
        let mut active = false;
        for &x in &verts {
            let val = h.n.dot(&x);
            if val >= h.c - tight {
                active = true;
                break;
            }
        }
        keep[i] = active;
    }
    let pruned: Vec<Hs4> = tmp
        .into_iter()
        .zip(keep)
        .filter_map(|(h, k)| if k { Some(h) } else { None })
        .collect();
    pruned
}

impl Poly4 {
    /// Check canonical invariants:
    /// - non-empty H-rep
    /// - unit normals (||n||≈1)
    /// - convexity (all vertices satisfy all half-spaces)
    /// - bounded (has vertices)
    /// - every facet is near-active on some vertex (no redundants)
    pub fn check_canonical(&mut self) -> Result<(), String> {
        if self.h.is_empty() {
            return Err("empty H-representation".into());
        }
        for (i, h) in self.h.iter().enumerate() {
            let nrm = h.n.norm();
            if (nrm - 1.0).abs() > 1e-8 {
                return Err(format!("facet {} has non-unit normal (||n||={})", i, nrm));
            }
        }
        // Ensure vertices (boundedness)
        self.ensure_vertices_from_h();
        if self.v.is_empty() {
            return Err("polytope appears unbounded or degenerate (no vertices)".into());
        }
        // Convexity
        if !self.is_convex() {
            return Err("convexity check failed".into());
        }
        // Facet support (no redundants)
        let tight = super::cfg::TIGHT_EPS;
        for (i, h) in self.h.iter().enumerate() {
            let mut active = false;
            for &x in &self.v {
                let val = h.n.dot(&x);
                if val >= h.c - tight {
                    active = true;
                    break;
                }
            }
            if !active {
                return Err(format!("facet {} not supporting (redundant)", i));
            }
        }
        Ok(())
    }

    #[inline]
    pub fn from_h(h: Vec<Hs4>) -> Self {
        let h_canon = canonicalize_h_strict(h);
        Self {
            h: h_canon,
            v: Vec::new(),
        }
    }
    #[inline]
    pub fn from_v(v: Vec<Vector4<f64>>) -> Self {
        Self { h: Vec::new(), v }
    }

    /// Append inequality (intersection).
    #[inline]
    pub fn intersect_halfspace(&mut self, hs: Hs4) {
        // Normalize and append; redundancy and duplicates are removed by later canonicalization.
        if let Some(hn) = hs.normalized() {
            self.h.push(hn);
        }
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
        self.h = canonicalize_h_strict(hs);
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
            // Renormalize to unit normal; skip degenerate.
            if let Some(h_norm) = Hs4::new(n_new, c_new).normalized() {
                out_h.push(h_norm);
            }
        }
        let mut out_v = Vec::with_capacity(self.v.len());
        for &v in &self.v {
            out_v.push(m * v + t);
        }
        Some(Self { h: out_h, v: out_v })
    }
}
