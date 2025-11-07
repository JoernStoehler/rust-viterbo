//! Strict, ordered H-representation in 2D (Poly2).
//!
//! Purpose
//! - Provide a single strict, angle‑ordered H‑rep (`Poly2`) with unit normals,
//!   coalesced parallels, and numerically explicit operations.
//!
//! Why this design
//! - Aligns with the oriented‑edge algorithm (push‑forward + HPI).
//! - Stable ordering by angle plus coalescing gives fast merges and predictable
//!   numerics for downstream algorithms.
//!
//! References
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md
//! - Code cross-refs: `types::{Hs2, Aff2, GeomCfg}`, `util::{angle_of, canonicalize_unit}`

use nalgebra::Vector2;

use super::types::Hs2;
use super::util::{angle_of, canonicalize_unit};
use super::Aff2;

/// Strict, ordered H-representation in 2D.
///
/// Invariants:
/// - Unit normals (||n||=1).
/// - Angle-sorted by atan2(n.y, n.x) (stable).
/// - Parallels coalesced (keep most restrictive c for each direction).
#[derive(Clone, Debug, Default)]
pub struct Poly2 {
    pub hs: Vec<Hs2>,
}

impl Poly2 {
    /// Insert a half-space and preserve invariants (binary search by angle, coalesce parallels).
    pub fn insert_halfspace(&mut self, h: Hs2) {
        if let Some((n, c)) = canonicalize_unit(h.n, h.c) {
            let h = Hs2::new(n, c);
            let key = angle_of(n);
            // binary search by angle
            let mut lo = 0usize;
            let mut hi = self.hs.len();
            while lo < hi {
                let mid = (lo + hi) / 2;
                let am = angle_of(self.hs[mid].n);
                if am <= key {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            // lo is insertion point
            if lo > 0 && (self.hs[lo - 1].n - n).norm() < 1e-9 {
                if c < self.hs[lo - 1].c {
                    self.hs[lo - 1].c = c;
                }
                return;
            }
            if lo < self.hs.len() && (self.hs[lo].n - n).norm() < 1e-9 {
                if c < self.hs[lo].c {
                    self.hs[lo].c = c;
                }
                return;
            }
            self.hs.insert(lo, h);
        }
    }

    /// Intersect with another strict poly (merge two sorted streams + coalesce).
    pub fn intersect(&self, other: &Poly2) -> Poly2 {
        let mut i = 0usize;
        let mut j = 0usize;
        let mut out: Vec<Hs2> = Vec::with_capacity(self.hs.len() + other.hs.len());
        while i < self.hs.len() && j < other.hs.len() {
            let ai = angle_of(self.hs[i].n);
            let bj = angle_of(other.hs[j].n);
            if (ai - bj).abs() < 1e-12 {
                // same direction: coalesce (keep min c)
                let c = self.hs[i].c.min(other.hs[j].c);
                push_or_coalesce(&mut out, self.hs[i].n, c);
                i += 1;
                j += 1;
            } else if ai < bj {
                push_or_coalesce(&mut out, self.hs[i].n, self.hs[i].c);
                i += 1;
            } else {
                push_or_coalesce(&mut out, other.hs[j].n, other.hs[j].c);
                j += 1;
            }
        }
        while i < self.hs.len() {
            push_or_coalesce(&mut out, self.hs[i].n, self.hs[i].c);
            i += 1;
        }
        while j < other.hs.len() {
            push_or_coalesce(&mut out, other.hs[j].n, other.hs[j].c);
            j += 1;
        }
        Poly2 { hs: out }
    }

    /// Membership check with custom slack (eps).
    ///
    /// Why (conservativeness policy):
    /// - Use `eps > 0` to be permissive (enlarge feasible region); e.g., for emptiness
    ///   checks, if the enlarged region is empty, the true region is certainly empty.
    /// - Use `eps < 0` to be strict (shrink the region); e.g., to certify non‑emptiness
    ///   robustly, if the shrunken region is non‑empty, the true region is certainly non‑empty.
    #[inline]
    pub fn contains_eps(&self, p: Vector2<f64>, eps: f64) -> bool {
        self.hs.iter().all(|h| h.satisfies_eps(p, eps))
    }

    /// Affine push-forward; result remains strict (re-normalize, sort, coalesce).
    pub fn push_forward(&self, f: &Aff2) -> Option<Poly2> {
        let minv = f.m.try_inverse()?;
        let mut tmp: Vec<Hs2> = Vec::with_capacity(self.hs.len());
        for h in &self.hs {
            let n_new = h.n.transpose() * minv;
            let n_new = Vector2::new(n_new[(0, 0)], n_new[(0, 1)]);
            let c_new = h.c + n_new.dot(&f.t);
            if let Some((n, c)) = canonicalize_unit(n_new, c_new) {
                tmp.push(Hs2::new(n, c));
            }
        }
        tmp.sort_by(|a, b| {
            let aa = angle_of(a.n);
            let bb = angle_of(b.n);
            aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut out = Vec::with_capacity(tmp.len());
        for h in tmp {
            push_or_coalesce(&mut out, h.n, h.c);
        }
        Some(Poly2 { hs: out })
    }

    /// Intersection of half-spaces using deque sweep on angle-sorted, coalesced constraints.
    ///
    /// eps policy:
    /// - `eps > 0` enlarges all half-spaces (c → c+eps): conservative for declaring emptiness.
    /// - `eps < 0` shrinks all half-spaces: conservative for certifying non‑emptiness.
    pub fn halfspace_intersection_eps(&self, eps: f64) -> HalfspaceIntersection {
        hsi_ordered(&self.hs, eps)
    }

    /// Shorthand for `halfspace_intersection_eps(0.0)`.
    #[inline]
    pub fn halfspace_intersection(&self) -> HalfspaceIntersection {
        self.halfspace_intersection_eps(0.0)
    }

    /// Emptiness with signed eps convention:
    /// - eps > 0 enlarges all half-spaces (conservative for declaring empty).
    /// - eps < 0 shrinks all half-spaces (conservative for certifying non‑empty).
    #[inline]
    pub fn is_empty_eps(&self, eps: f64) -> bool {
        self.halfspace_intersection_eps(eps).is_empty()
    }

    /// Return a new poly with one additional cut (half-space) applied.
    #[inline]
    pub fn with_cut(&self, cut: Hs2) -> Poly2 {
        let mut out = self.clone();
        out.insert_halfspace(cut);
        out
    }
}

/// HPI result: empty, unbounded, or vertices.
#[derive(Clone, Debug)]
pub enum HalfspaceIntersection {
    Empty,
    Unbounded,
    Bounded(Vec<Vector2<f64>>),
}
impl HalfspaceIntersection {
    #[inline]
    pub fn is_empty(&self) -> bool {
        matches!(self, HalfspaceIntersection::Empty)
    }
    #[inline]
    pub fn is_bounded(&self) -> bool {
        matches!(self, HalfspaceIntersection::Bounded(_))
    }
    #[inline]
    pub fn vertices(self) -> Option<Vec<Vector2<f64>>> {
        if let HalfspaceIntersection::Bounded(v) = self {
            Some(v)
        } else {
            None
        }
    }
}

fn hsi_ordered(hs: &[Hs2], eps: f64) -> HalfspaceIntersection {
    use std::collections::VecDeque;
    if hs.is_empty() {
        return HalfspaceIntersection::Unbounded;
    }
    // Fast contradiction check for any opposite parallel pair (interval test):
    // For n·x <= c1 and (-n)·x <= c2, s := n·x ∈ [max(-c1, -c2), min(c1, c2)].
    // Empty iff max(-c1, -c2) > min(c1, c2).
    let angles: Vec<f64> = hs.iter().map(|h| angle_of(h.n)).collect();
    for (i, hi) in hs.iter().enumerate() {
        let ai = angles[i];
        let target = wrap_angle(ai + std::f64::consts::PI);
        // binary search nearest to target
        let mut lo = 0usize;
        let mut hi_idx = angles.len();
        while lo < hi_idx {
            let mid = (lo + hi_idx) / 2;
            if angles[mid] < target {
                lo = mid + 1;
            } else {
                hi_idx = mid;
            }
        }
        if lo < angles.len() && (angles[lo] - target).abs() < 1e-12 {
            let c1 = hi.c;
            let c2 = hs[lo].c;
            if (-c1).max(-c2) > c1.min(c2) {
                return HalfspaceIntersection::Empty;
            }
        }
    }
    let mut dq: VecDeque<usize> = VecDeque::new();
    let inter =
        |i1: usize, i2: usize| -> Option<Vector2<f64>> { line_intersection(hs[i1], hs[i2]) };

    for (i, h) in hs.iter().enumerate() {
        while dq.len() >= 2 {
            let l1 = dq[dq.len() - 2];
            let l2 = dq[dq.len() - 1];
            if let Some(p) = inter(l1, l2) {
                if h.satisfies_eps(p, eps) {
                    break;
                }
            }
            dq.pop_back();
        }
        while dq.len() >= 2 {
            let f1 = dq[0];
            let f2 = dq[1];
            if let Some(p) = inter(f1, f2) {
                if h.satisfies_eps(p, eps) {
                    break;
                }
            }
            dq.pop_front();
        }
        dq.push_back(i);
    }
    while dq.len() >= 3 {
        let l1 = dq[dq.len() - 2];
        let l2 = dq[dq.len() - 1];
        if let Some(p) = line_intersection(hs[l1], hs[l2]) {
            if hs[dq[0]].satisfies_eps(p, eps) {
                break;
            }
        }
        dq.pop_back();
    }
    while dq.len() >= 3 {
        let f1 = dq[0];
        let f2 = dq[1];
        if let Some(p) = line_intersection(hs[f1], hs[f2]) {
            if hs[dq[dq.len() - 1]].satisfies_eps(p, eps) {
                break;
            }
        }
        dq.pop_front();
    }
    if dq.is_empty() {
        return HalfspaceIntersection::Empty;
    }
    if dq.len() < 3 {
        return HalfspaceIntersection::Unbounded;
    }
    let m = dq.len();
    let mut verts = Vec::with_capacity(m);
    for k in 0..m {
        let i1 = dq[k];
        let i2 = dq[(k + 1) % m];
        if let Some(p) = line_intersection(hs[i1], hs[i2]) {
            verts.push(p);
        } else {
            return HalfspaceIntersection::Unbounded;
        }
    }
    if verts.len() >= 3 {
        HalfspaceIntersection::Bounded(verts)
    } else {
        HalfspaceIntersection::Unbounded
    }
}

#[inline]
pub(crate) fn wrap_angle(a: f64) -> f64 {
    let mut x = a;
    while x <= -std::f64::consts::PI {
        x += 2.0 * std::f64::consts::PI;
    }
    while x > std::f64::consts::PI {
        x -= 2.0 * std::f64::consts::PI;
    }
    x
}
#[inline]
pub(crate) fn push_or_coalesce(out: &mut Vec<Hs2>, n: Vector2<f64>, c: f64) {
    if let Some(last) = out.last_mut() {
        if (last.n - n).norm() < 1e-9 {
            if c < last.c {
                last.c = c;
            }
            return;
        }
    }
    out.push(Hs2::new(n, c));
}
fn line_intersection(h1: Hs2, h2: Hs2) -> Option<Vector2<f64>> {
    let a = nalgebra::matrix![h1.n.x, h1.n.y; h2.n.x, h2.n.y];
    let det = a.determinant();
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = a.try_inverse()?;
    let rhs = Vector2::new(h1.c, h2.c);
    Some(inv * rhs)
}
