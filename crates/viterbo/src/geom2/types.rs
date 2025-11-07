//! Basic 2D types and tolerances used by strict H-representations.
//!
//! - `GeomCfg`: centralizes epsilons for determinant, feasibility, and tau checks.
//! - `Hs2`: closed half‑space `n·x <= c` with helper predicates.
//! - `Affine2`, `Aff1`: 2D affine map and 1D affine functional used by search.
//!
//! References
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md
//! - Code cross-refs: `ordered::Poly2`, `solvers::{rotation_angle,fixed_point_in_poly}`

use nalgebra::{Matrix2, Vector2};

/// Geometry configuration (tolerances).
///
/// TH: capacity-oriented-edge (numeric robustness)
#[derive(Clone, Copy, Debug)]
pub struct GeomCfg {
    pub eps_det: f64,
    pub eps_feas: f64,
    pub eps_tau: f64,
}

impl Default for GeomCfg {
    fn default() -> Self {
        Self {
            eps_det: 1e-12,
            eps_feas: 1e-9,
            eps_tau: 1e-9,
        }
    }
}

/// Closed half‑space `n · x <= c` (no normalization required here).
#[derive(Clone, Copy, Debug)]
pub struct Hs2 {
    pub n: Vector2<f64>,
    pub c: f64,
}

impl Hs2 {
    #[inline]
    pub fn new(n: Vector2<f64>, c: f64) -> Self {
        Self { n, c }
    }
    #[inline]
    pub fn satisfies_eps(&self, p: Vector2<f64>, eps: f64) -> bool {
        self.n.dot(&p) <= self.c + eps
    }
}

/// 2D affine map: `x ↦ M x + t`.
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
    #[inline]
    pub fn inverse(&self) -> Option<Self> {
        self.m.try_inverse().map(|minv| Self {
            m: minv,
            t: -minv * self.t,
        })
    }
    #[inline]
    pub fn is_orientation_preserving(&self) -> bool {
        self.m.determinant() > 0.0
    }
    /// Polar rotation factor Q from SVD (if orientation-preserving). None if `det(Q)<0`.
    pub fn polar_rotation(&self) -> Option<Matrix2<f64>> {
        use nalgebra::SVD;
        let svd = SVD::new(self.m, true, true);
        let u = svd.u?;
        let vt = svd.v_t?;
        let q = u * vt;
        if q.determinant() < 0.0 {
            None
        } else {
            Some(q)
        }
    }
}

/// 1D affine functional `A(z) = a·z + b`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aff1 {
    pub a: Vector2<f64>,
    pub b: f64,
}

impl Aff1 {
    #[inline]
    pub fn eval(&self, z: Vector2<f64>) -> f64 {
        self.a.dot(&z) + self.b
    }
    /// A∘φ where φ(z)=Mz+t → (M^T a, a·t + b)
    #[inline]
    pub fn compose_with_affine2(&self, phi: &Affine2) -> Aff1 {
        let a_new = phi.m.transpose() * self.a;
        let b_new = self.a.dot(&phi.t) + self.b;
        Aff1 { a: a_new, b: b_new }
    }
    /// A∘φ^{-1} if invertible.
    #[inline]
    pub fn compose_with_inv_affine2(&self, phi: &Affine2) -> Option<Aff1> {
        let inv = phi.inverse()?;
        Some(self.compose_with_affine2(&inv))
    }
    /// Pointwise addition.
    #[inline]
    pub fn add(&self, other: &Aff1) -> Aff1 {
        Aff1 {
            a: self.a + other.a,
            b: self.b + other.b,
        }
    }
    /// Half-space cut { A(z) ≤ A_best }.
    #[inline]
    pub fn to_cut(&self, a_best: f64) -> Hs2 {
        Hs2::new(self.a, a_best - self.b)
    }
}

impl std::ops::Add for Aff1 {
    type Output = Aff1;
    #[inline]
    fn add(self, rhs: Aff1) -> Self::Output {
        Aff1 {
            a: self.a + rhs.a,
            b: self.b + rhs.b,
        }
    }
}
impl std::ops::Neg for Aff1 {
    type Output = Aff1;
    #[inline]
    fn neg(self) -> Self::Output {
        Aff1 {
            a: -self.a,
            b: -self.b,
        }
    }
}
