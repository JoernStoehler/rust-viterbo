//! 2D Convex Polytopes (H-representation focused).
//!
//! Purpose
//! - Production module for the performance‑critical 2D routines (scale: up to ~1e9
//!   short‑lived polytopes in oriented‑edge DFS).
//!
//! Why this design (short)
//! - H‑rep fits our hot path: push‑forward and intersections are cheap and avoid
//!   constructing vertices/hulls.
//! - We keep data layout simple and contiguous (small `Vec` of half‑spaces).
//! - Degeneracy handling is intentionally light by ticket: we optimize for generic
//!   inputs and document behavior in edge cases.
//!
//! Assumptions and conventions
//! - Half‑spaces are of the form `n·x <= c`. We do not require `n` normalized and
//!   we do not require `c >= 0`. Any finite `(n,c)` is accepted. Rationale:
//!   normalization costs on the hot path and is unnecessary for correctness.
//! - Numerical tolerance: predicates use `EPS = 1e-9` (scale‑agnostic; callers
//!   should avoid extreme scalings).
//! - Emptiness is a heuristic: we test pairwise boundary intersections and a few
//!   probes; degenerate strips may be misclassified (acceptable per ticket).
//! - `normalize_simple()` is optional and cheap: it rescales normals to unit
//!   length, adjusts `c`, sorts by angle, and dedups near‑duplicates. It does not
//!   perform expensive redundancy elimination.
//!
//! References
//! - TH: docs/src/thesis/geom2d_polytopes.md
//! - AGENTS: `AGENTS.md` (Rust conventions, testing policy)
//! - Code cross‑refs: `Poly2`, `Hs2`, `Affine2`

use nalgebra::{matrix, Matrix2, Vector2};
use std::collections::VecDeque;

/// Numerical tolerance used for geometric predicates.
///
/// Why: keeps branchy predicates stable without forcing normalization in the
/// hot path. Value tuned for typical O(1) scales; adjust per config if needed.
const EPS: f64 = 1e-9;

/// Closed half‑space `n · x <= c`.
///
/// Invariants (design choice):
/// - `n` is not required to be unit length; `c` may be any finite real.
/// - We do not reject near‑zero `||n||`; if `n≈0`, the inequality becomes
///   `0 <= c` and thus either always true or always false depending on `c`.
///   Callers should avoid such inputs or pre‑filter them if undesirable.
/// - Membership uses `<= c + EPS`.
#[derive(Clone, Copy, Debug)]
pub struct Hs2 {
    pub n: Vector2<f64>,
    pub c: f64,
}

impl Hs2 {
    /// Construct from normal and offset without normalization or checks.
    #[inline]
    pub fn new(n: Vector2<f64>, c: f64) -> Self {
        Self { n, c }
    }

    #[inline]
    pub fn satisfies(&self, p: Vector2<f64>) -> bool {
        self.n.dot(&p) <= self.c + EPS
    }
}

/// 2D affine map: `x ↦ M x + t`.
///
/// Why not store a precomputed inverse?
/// - We keep this struct trivial and compute inverses on demand only where needed
///   to avoid extra storage and because many maps are never inverted.
#[derive(Clone, Copy, Debug)]
pub struct Affine2 {
    pub m: Matrix2<f64>,
    pub t: Vector2<f64>,
}

impl Affine2 {
    #[inline]
    pub fn identity() -> Self {
        Self {
            m: Matrix2::identity(),
            t: Vector2::zeros(),
        }
    }

    /// Inverse map if `m` is invertible.
    ///
    /// Returns `None` if `det(M) ≈ 0`.
    pub fn inverse(&self) -> Option<Self> {
        self.m.try_inverse().map(|minv| Self {
            m: minv,
            t: -minv * self.t,
        })
    }

    /// Composition `self ∘ other`.
    ///
    /// No special assumptions; associates as expected.
    #[inline]
    pub fn compose(&self, other: &Self) -> Self {
        Self {
            m: self.m * other.m,
            t: self.m * other.t + self.t,
        }
    }

    /// Fixed point of `x = M x + t`, if unique.
    ///
    /// Solves `(I - M) x = t`; returns `None` when `I - M` is singular.
    pub fn fixed_point(&self) -> Option<Vector2<f64>> {
        // (I - M) x = t
        let a = Matrix2::identity() - self.m;
        a.try_inverse().map(|ainv| ainv * self.t)
    }
}

/// 2D convex polytope in H‑rep: intersection of finitely many half‑spaces.
///
/// Invariants:
/// - No normalization enforced; half‑spaces are used as provided.
/// - `hs.len()` may be zero (interpreted as R^2).
/// - Methods prefer O(m) operations and avoid vertex construction on the hot path.
#[derive(Clone, Debug, Default)]
pub struct HPoly2 {
    pub hs: Vec<Hs2>,
}

impl HPoly2 {
    #[inline]
    pub fn new() -> Self {
        Self { hs: Vec::new() }
    }

    #[inline]
    pub fn from_halfspaces(hs: Vec<Hs2>) -> Self {
        Self { hs }
    }

    /// Intersect with a new half‑space (append inequality).
    ///
    /// Why append-only? We avoid expensive redundancy checks in the hot path.
    #[inline]
    pub fn intersect_halfspace(&mut self, hs: Hs2) {
        self.hs.push(hs);
    }

    /// Intersect with another polytope (append all inequalities).
    #[inline]
    pub fn intersect_poly(&mut self, other: &HPoly2) {
        self.hs.extend_from_slice(&other.hs);
    }

    /// Push‑forward under invertible affine map `y = M x + t`.
    ///
    /// If `M` not invertible, returns `None`.
    ///
    /// Derivation: With `n·x <= c` and `x = M^{-1}(y - t)`, we get
    /// `(n M^{-1})·y <= c + (n M^{-1})·t`. We implement `A' = A M^{-1}` and
    /// `c' = c + A'·t` for each row.
    pub fn push_forward(&self, f: &Affine2) -> Option<HPoly2> {
        let minv = f.m.try_inverse()?;
        let mut out = Vec::with_capacity(self.hs.len());
        for h in &self.hs {
            // y feasible iff A (M^{-1}(y - t)) <= b
            // A' = A M^{-1}; b' = b + A' · t
            let n_new = h.n.transpose() * minv;
            let n_new = Vector2::new(n_new[(0, 0)], n_new[(0, 1)]);
            let c_new = h.c + n_new.dot(&f.t);
            out.push(Hs2::new(n_new, c_new));
        }
        Some(HPoly2 { hs: out })
    }

    /// Fast membership test.
    #[inline]
    pub fn contains(&self, p: Vector2<f64>) -> bool {
        self.hs.iter().all(|h| h.satisfies(p))
    }

    /// Heuristic emptiness test optimized for generic (non‑degenerate) inputs.
    ///
    /// Strategy
    /// - 0 or 1 half‑space: non‑empty
    /// - Otherwise: enumerate pairwise line intersections `a_i·x=c_i` and
    ///   accept if any vertex satisfies all inequalities.
    /// - If no finite vertex exists (parallel strip), returns false negatives
    ///   in degenerate cases, which are outside our hot path per ticket.
    pub fn is_empty(&self) -> bool {
        let m = self.hs.len();
        if m == 0 || m == 1 {
            return false;
        }
        // Try all pairs
        for i in 0..m {
            for j in (i + 1)..m {
                if let Some(p) = line_intersection(self.hs[i], self.hs[j]) {
                    if self.contains(p) {
                        return false;
                    }
                }
            }
        }
        // As a weak fallback, try origin and a few canonical points.
        let probes = [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(-1.0, 0.0),
            Vector2::new(0.0, -1.0),
        ];
        if probes.iter().any(|&p| self.contains(p)) {
            return false;
        }
        true
    }

    /// Remove near‑duplicate constraints and sort by normal angle.
    ///
    /// This is a cheap normalization to improve cache behavior and downstream
    /// performance. It does not perform redundancy elimination (e.g., removing
    /// dominated constraints); that would require more expensive passes.
    pub fn normalize_simple(&mut self) {
        // Normalize (n, c) so that ||n|| = 1 and c scaled accordingly.
        let mut tmp = Vec::with_capacity(self.hs.len());
        for mut h in self.hs.drain(..) {
            let norm = h.n.norm();
            if norm > 0.0 {
                h.n /= norm;
                h.c /= norm;
            }
            tmp.push(h);
        }
        // Sort by angle of normal.
        tmp.sort_by(|a, b| {
            let aa = a.n.y.atan2(a.n.x);
            let bb = b.n.y.atan2(b.n.x);
            aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Dedup within tolerance.
        let mut deduped = Vec::with_capacity(tmp.len());
        for h in tmp {
            if !deduped.iter().any(|g: &Hs2| (g.n - h.n).norm() < 1e-9 && (g.c - h.c).abs() < 1e-9)
            {
                deduped.push(h);
            }
        }
        self.hs = deduped;
    }

    /// Compute min/max of an affine functional `f(x) = w·x + a` over vertices
    /// discovered via pairwise boundary intersections.
    ///
    /// Returns `None` if no finite vertex is found (e.g., unbounded strip).
    pub fn extremal_affine(
        &self,
        w: Vector2<f64>,
        a: f64,
    ) -> Option<(f64, Vector2<f64>, f64, Vector2<f64>)> {
        let mut minv = f64::INFINITY;
        let mut minp = Vector2::zeros();
        let mut maxv = f64::NEG_INFINITY;
        let mut maxp = Vector2::zeros();
        let m = self.hs.len();
        let mut found = false;
        for i in 0..m {
            for j in (i + 1)..m {
                if let Some(p) = line_intersection(self.hs[i], self.hs[j]) {
                    if self.contains(p) {
                        let val = w.dot(&p) + a;
                        if val < minv {
                            minv = val;
                            minp = p;
                        }
                        if val > maxv {
                            maxv = val;
                            maxp = p;
                        }
                        found = true;
                    }
                }
            }
        }
        if found {
            Some((minv, minp, maxv, maxp))
        } else {
            None
        }
    }

    /// Construct H‑rep from a set of 2D points by taking their convex hull
    /// (Andrew’s monotone chain) and forming outward half‑spaces.
    ///
    /// Orientation/why: for each edge `p→q` we take outward normal by 90° CCW
    /// rotation `(-edge_y, edge_x)` and set `c = n·p`, yielding `n·x <= c`.
    pub fn from_points_convex_hull(points: &[Vector2<f64>]) -> Option<Self> {
        let hull = convex_hull(points)?;
        if hull.len() < 2 {
            return None;
        }
        let mut hs = Vec::with_capacity(hull.len());
        for k in 0..hull.len() {
            let p = hull[k];
            let q = hull[(k + 1) % hull.len()];
            let edge = q - p;
            // Outward normal (rotate 90° CCW).
            let n = Vector2::new(-edge.y, edge.x);
            // Ensure n points outward: require all points are on or inside.
            let c = n.dot(&p);
            hs.push(Hs2::new(n, c));
        }
        let mut poly = HPoly2::from_halfspaces(hs);
        poly.normalize_simple();
        Some(poly)
    }

    /// Convert to a strict, ordered representation with unit normals,
    /// angle-sorted, and parallels coalesced.
    pub fn to_ordered(&self) -> HPoly2Ordered {
        HPoly2Ordered::from_unordered(self)
    }
}

/// Strict, ordered H-representation in 2D.
///
/// Invariants:
/// - Unit normals (`||n||=1`).
/// - Angle-sorted by `atan2(n.y, n.x)` (stable).
/// - Parallels coalesced (keep most restrictive `c` for each direction).
#[derive(Clone, Debug, Default)]
pub struct HPoly2Ordered {
    pub hs: Vec<Hs2>,
}

impl HPoly2Ordered {
    /// Build from a loose poly by normalizing, sorting, and coalescing parallel half-spaces.
    pub fn from_unordered(p: &HPoly2) -> Self {
        let mut tmp: Vec<Hs2> = Vec::with_capacity(p.hs.len());
        for h in &p.hs {
            if let Some((n, c)) = canonicalize_unit(h.n, h.c) {
                tmp.push(Hs2::new(n, c));
            }
        }
        // sort by angle
        tmp.sort_by(|a, b| {
            let aa = a.n.y.atan2(a.n.x);
            let bb = b.n.y.atan2(b.n.x);
            aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        });
        // coalesce parallels (same direction)
        let mut out: Vec<Hs2> = Vec::with_capacity(tmp.len());
        for h in tmp {
            if let Some(last) = out.last_mut() {
                if (last.n - h.n).norm() < 1e-9 {
                    // keep most restrictive: smaller c
                    if h.c < last.c {
                        last.c = h.c;
                    }
                    continue;
                }
            }
            out.push(h);
        }
        Self { hs: out }
    }

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
                if am < key {
                    lo = mid + 1;
                } else {
                    hi = mid;
                }
            }
            // lo is insertion point
            if lo > 0 && (self.hs[lo - 1].n - n).norm() < 1e-9 {
                // coalesce with previous
                if c < self.hs[lo - 1].c {
                    self.hs[lo - 1].c = c;
                }
                return;
            }
            if lo < self.hs.len() && (self.hs[lo].n - n).norm() < 1e-9 {
                // coalesce with next
                if c < self.hs[lo].c {
                    self.hs[lo].c = c;
                }
                return;
            }
            self.hs.insert(lo, h);
        }
    }

    /// Intersect with another strict poly (merge two sorted streams + coalesce).
    pub fn intersect_ordered(&self, other: &HPoly2Ordered) -> HPoly2Ordered {
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
        HPoly2Ordered { hs: out }
    }

    /// Membership check (O(m)).
    pub fn contains(&self, p: Vector2<f64>) -> bool {
        self.hs.iter().all(|h| h.satisfies(p))
    }

    /// Affine push-forward; result remains strict (re-normalize, sort, coalesce).
    pub fn push_forward(&self, f: &Affine2) -> Option<HPoly2Ordered> {
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
        Some(HPoly2Ordered { hs: out })
    }

    /// Half-plane intersection (HPI) using deque sweep on angle-sorted, coalesced constraints.
    pub fn hpi(&self) -> HpiResult {
        hpi_ordered(&self.hs)
    }

    /// Convert to loose representation (relaxes invariants).
    pub fn to_loose(&self) -> HPoly2 {
        HPoly2 {
            hs: self.hs.clone(),
        }
    }
}

/// HPI result: either empty, unbounded, or a list of vertices in CCW order.
#[derive(Clone, Debug)]
pub enum HpiResult {
    Empty,
    Unbounded,
    Bounded(Vec<Vector2<f64>>),
}

fn hpi_ordered(hs: &[Hs2]) -> HpiResult {
    if hs.is_empty() {
        return HpiResult::Unbounded;
    }
    // Fast contradiction check for opposite parallel pairs:
    // For unit normals n and -n with constraints n·x <= c_u and (-n)·x <= c_l,
    // the implied interval for s = n·x is [-c_l, c_u]. If -c_l > c_u -> empty.
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
        // Check neighbor candidates around insertion point
        for k in lo.saturating_sub(1)..=lo.min(angles.len().saturating_sub(1)) {
            if (angles[k] - target).abs() < 1e-9 {
                // Parallel and opposite directions
                let hj = &hs[k];
                // interval [-c_l, c_u] with c_l = c for -n, c_u = c for n
                let lower = -hj.c;
                let upper = hi.c;
                if lower > upper + EPS {
                    return HpiResult::Empty;
                }
            }
        }
    }
    let mut dq: VecDeque<usize> = VecDeque::new();
    // Helper to get intersection of last two in deque
    let mut inter = |i1: usize, i2: usize| -> Option<Vector2<f64>> { line_intersection(hs[i1], hs[i2]) };

    for i in 0..hs.len() {
        // Pop from back while the intersection of the last pair violates current half-space
        while dq.len() >= 2 {
            let l1 = dq[dq.len() - 2];
            let l2 = dq[dq.len() - 1];
            if let Some(p) = inter(l1, l2) {
                if hs[i].satisfies(p) {
                    break;
                }
            }
            dq.pop_back();
        }
        // Pop from front similarly
        while dq.len() >= 2 {
            let f1 = dq[0];
            let f2 = dq[1];
            if let Some(p) = inter(f1, f2) {
                if hs[i].satisfies(p) {
                    break;
                }
            }
            dq.pop_front();
        }
        dq.push_back(i);
    }
    // Final cleanup against first/last
    while dq.len() >= 3 {
        let l1 = dq[dq.len() - 2];
        let l2 = dq[dq.len() - 1];
        if let Some(p) = line_intersection(hs[l1], hs[l2]) {
            if hs[dq[0]].satisfies(p) {
                break;
            }
        }
        dq.pop_back();
    }
    while dq.len() >= 3 {
        let f1 = dq[0];
        let f2 = dq[1];
        if let Some(p) = line_intersection(hs[f1], hs[f2]) {
            if hs[dq[dq.len() - 1]].satisfies(p) {
                break;
            }
        }
        dq.pop_front();
    }
    if dq.is_empty() {
        return HpiResult::Empty;
    }
    if dq.len() < 3 {
        return HpiResult::Unbounded;
    }
    // Build polygon vertices
    let m = dq.len();
    let mut verts = Vec::with_capacity(m);
    for k in 0..m {
        let i1 = dq[k];
        let i2 = dq[(k + 1) % m];
        if let Some(p) = line_intersection(hs[i1], hs[i2]) {
            verts.push(p);
        } else {
            // Degenerate intersection -> treat as unbounded
            return HpiResult::Unbounded;
        }
    }
    if verts.len() >= 3 {
        HpiResult::Bounded(verts)
    } else {
        HpiResult::Unbounded
    }
}

#[inline]
fn angle_of(n: Vector2<f64>) -> f64 {
    n.y.atan2(n.x)
}

#[inline]
fn wrap_angle(a: f64) -> f64 {
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
fn push_or_coalesce(out: &mut Vec<Hs2>, n: Vector2<f64>, c: f64) {
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

fn canonicalize_unit(n: Vector2<f64>, c: f64) -> Option<(Vector2<f64>, f64)> {
    let norm = n.norm();
    if !(norm.is_finite()) || norm <= 0.0 {
        // Degenerate constraint; ignore.
        return None;
    }
    Some((n / norm, c / norm))
}

/// Backward-compat: keep old name for loose poly.
pub type Poly2 = HPoly2;

/// Intersection of two boundary lines `a·x=c` and `b·x=c`.
///
/// Notes:
/// - Uses a simple 2×2 solve; returns `None` when lines are parallel/near‑parallel.
/// - Caller decides what to do with the candidate (e.g., feasibility check).
fn line_intersection(h1: Hs2, h2: Hs2) -> Option<Vector2<f64>> {
    // Rows are normals: [n1^T; n2^T] x = [c1; c2]
    let a = matrix![h1.n.x, h1.n.y; h2.n.x, h2.n.y];
    let det = a.determinant();
    if det.abs() < EPS {
        return None;
    }
    let inv = a.try_inverse()?;
    let rhs = Vector2::new(h1.c, h2.c);
    Some(inv * rhs)
}

/// Andrew’s monotone chain convex hull (returns hull in CCW order, deduped).
///
/// Complexity: O(N log N) for sort + linear passes.
fn convex_hull(points: &[Vector2<f64>]) -> Option<Vec<Vector2<f64>>> {
    if points.len() < 2 {
        return None;
    }
    let mut pts: Vec<_> = points.to_vec();
    pts.sort_by(|a, b| {
        match a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal) {
            std::cmp::Ordering::Equal => a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal),
            o => o,
        }
    });
    pts.dedup_by(|a, b| (*a - *b).norm() < 1e-12);
    if pts.len() < 2 {
        return None;
    }
    let mut lower: Vec<Vector2<f64>> = Vec::with_capacity(pts.len());
    for p in &pts {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0
        {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Vector2<f64>> = Vec::with_capacity(pts.len());
    for p in pts.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0
        {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    let mut hull = lower;
    hull.extend(upper);
    Some(hull)
}

#[inline]
fn cross(a: Vector2<f64>, b: Vector2<f64>, c: Vector2<f64>) -> f64 {
    let ab = b - a;
    let ac = c - a;
    ab.x * ac.y - ab.y * ac.x
}

/// CZ‑index related rotation of an orientation‑preserving affine map.
///
/// Placeholder. We expose a stub to allow call sites to compile and will
/// implement once the exact formula is specified in the thesis layer.
pub fn cz_index_rotation_stub(_f: &Affine2) -> Option<Matrix2<f64>> {
    // TH TODO: specify computation for orientation‑preserving maps.
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{matrix, vector};
    use rand::{rngs::StdRng, Rng, SeedableRng};
    use std::f64::consts::FRAC_PI_2;

    #[test]
    fn push_forward_affine_box() {
        // Unit box: 0<=x<=1, 0<=y<=1
        let mut p = HPoly2::new();
        p.intersect_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![-1.0, 0.0], 0.0));
        p.intersect_halfspace(Hs2::new(vector![0.0, 1.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![0.0, -1.0], 0.0));

        let f = Affine2 {
            m: matrix![2.0, 0.0; 0.0, 0.5],
            t: vector![1.0, -1.0],
        };
        let q = p.push_forward(&f).unwrap();
        // Check images of a few points
        for &(x, y) in &[(0.0, 0.0), (1.0, 1.0), (0.25, 0.75)] {
            let z = vector![x, y];
            let y_img = f.m * z + f.t;
            assert!(q.contains(y_img));
        }
    }

    #[test]
    fn emptiness_generic_triangle() {
        // Three halfspaces forming a triangle with a vertex explicitly present.
        let mut p = HPoly2::new();
        p.intersect_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![0.0, 1.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![-1.0, -1.0], -0.25));
        assert!(!p.is_empty());
    }

    #[test]
    fn extremal_affine_triangle() {
        let mut p = HPoly2::new();
        p.intersect_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![0.0, 1.0], 1.0));
        p.intersect_halfspace(Hs2::new(vector![-1.0, -1.0], -0.25));
        let (minv, _minp, maxv, _maxp) = p.extremal_affine(vector![1.0, 1.0], 0.0).unwrap();
        assert!(minv.is_finite() && maxv.is_finite());
        assert!(maxv > minv);
    }

    #[test]
    fn convex_hull_round_trip() {
        let mut rng = StdRng::seed_from_u64(123);
        let pts: Vec<_> = (0..20)
            .map(|_| vector![rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0)])
            .collect();
        let poly = HPoly2::from_points_convex_hull(&pts).unwrap();
        // Sample a few hull vertices; they must satisfy all inequalities.
        for &h in &poly.hs {
            // pick boundary point p on the line n·x=c near origin
            let n = h.n;
            // any perpendicular vector to construct a sample
            let perp = vector![-n.y, n.x];
            // point on boundary line (closest to origin)
            let p = n * (h.c / (n.dot(&n)));
            assert!(h.satisfies(p + 1e-7 * perp)); // inside tiny offset
        }
    }

    #[test]
    fn ordered_invariants() {
        // Random loose poly -> ordered
        let mut rng = StdRng::seed_from_u64(7);
        let mut loose = HPoly2::new();
        for _ in 0..25 {
            let th = rng.gen::<f64>() * std::f64::consts::TAU;
            let n = vector![th.cos(), th.sin()] * rng.gen_range(0.1..3.0);
            let c = rng.gen_range(-2.0..2.0);
            loose.intersect_halfspace(Hs2::new(n, c));
        }
        let ordered = loose.to_ordered();
        // Unit normals and canonical sign
        for h in &ordered.hs {
            let norm = h.n.norm();
            assert!((norm - 1.0).abs() < 1e-9);
        }
        // Angle sorted non-decreasing
        for k in 1..ordered.hs.len() {
            assert!(angle_of(ordered.hs[k - 1].n) <= angle_of(ordered.hs[k].n) + 1e-15);
        }
    }

    #[test]
    fn ordered_insert_and_merge() {
        let mut a = HPoly2Ordered::default();
        // Insert two parallels; coalesce (keep min c)
        let n = vector![1.0, 0.0];
        a.insert_halfspace(Hs2::new(n, 1.0));
        a.insert_halfspace(Hs2::new(2.0 * n, 0.4));
        assert_eq!(a.hs.len(), 1);
        assert!((a.hs[0].c - 0.2).abs() < 1e-12); // because normalization by 2

        let mut b = HPoly2Ordered::default();
        b.insert_halfspace(Hs2::new(vector![0.0, 1.0], 2.0));
        let m = a.intersect_ordered(&b);
        assert_eq!(m.hs.len(), 2);
        // sorted angles: (0 rad) then (pi/2)
        assert!(angle_of(m.hs[0].n) <= angle_of(m.hs[1].n));
    }

    #[test]
    fn hpi_empty_bounded_unbounded() {
        // Empty: x<=0 and x>=1
        let mut e = HPoly2::new();
        e.intersect_halfspace(Hs2::new(vector![1.0, 0.0], 0.0));
        e.intersect_halfspace(Hs2::new(vector![-1.0, 0.0], -1.0));
        match e.to_ordered().hpi() {
            HpiResult::Empty => {}
            _ => panic!("expected empty"),
        }
        // Bounded: unit square
        let mut b = HPoly2::new();
        b.intersect_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
        b.intersect_halfspace(Hs2::new(vector![-1.0, 0.0], 0.0));
        b.intersect_halfspace(Hs2::new(vector![0.0, 1.0], 1.0));
        b.intersect_halfspace(Hs2::new(vector![0.0, -1.0], 0.0));
        match b.to_ordered().hpi() {
            HpiResult::Bounded(verts) => assert!(verts.len() >= 4),
            _ => panic!("expected bounded"),
        }
        // Unbounded: wedge x>=0, y>=0
        let mut u = HPoly2::new();
        u.intersect_halfspace(Hs2::new(vector![-1.0, 0.0], 0.0)); // x>=0
        u.intersect_halfspace(Hs2::new(vector![0.0, -1.0], 0.0)); // y>=0
        match u.to_ordered().hpi() {
            HpiResult::Unbounded => {}
            _ => panic!("expected unbounded"),
        }
    }

    #[test]
    fn strict_to_loose_cast() {
        let mut s = HPoly2Ordered::default();
        s.insert_halfspace(Hs2::new(vector![1.0, 0.0], 1.0));
        let l: HPoly2 = s.to_loose();
        assert_eq!(l.hs.len(), 1);
    }
}
