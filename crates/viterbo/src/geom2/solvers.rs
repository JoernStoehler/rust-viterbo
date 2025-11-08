//! Small 2D solvers used by oriented‑edge search and experiments.
//!
//! - `rotation_angle`: orientation‑preserving polar factor angle/π in [0,1].
//! - `fixed_point_in_poly`: constrained fixed‑point solve with action minimization.
//!
//! References
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md
//! - Code cross-refs: `ordered::{Poly2,HalfspaceIntersection}`, `types::{Aff1,Aff2,GeomCfg}`
use nalgebra::{Matrix2, Vector2, SVD};

use super::{ordered::HalfspaceIntersection, ordered::Poly2, types::Aff1, types::GeomCfg, Aff2};

/// Rotation via polar factor (principal angle): returns angle/π in [0,1],
/// or None if orientation-reversing.
///
/// Definition (principal angle normalization)
/// - Let M be the 2×2 linear part of the affine map. Compute the polar
///   decomposition M = R S with R ∈ SO(2), S ≻ 0. Define rot(M) := arg(R) ∈ [0, π].
/// - We return ρ := rot(M)/π ∈ [0, 1].
/// - For generic (non‑degenerate) edges in our ω₀‑canonical charts, 0 < ρ < 1.
///   The only way to get ρ=0 is a true (or numerically near) identity step.
pub fn rotation_angle(f: &Aff2) -> Option<f64> {
    let svd = SVD::new(f.m, true, true);
    let u = svd.u?;
    let vt = svd.v_t?;
    let q = u * vt;
    let det_q = q.determinant();
    if !det_q.is_finite() {
        return None;
    }
    if det_q < 0.0 {
        return None;
    }
    let theta = q[(1, 0)].atan2(q[(0, 0)]);
    debug_assert!(
        (-std::f64::consts::PI..=std::f64::consts::PI).contains(&theta),
        "principal angle out of range"
    );
    let rho = theta.abs() / std::f64::consts::PI;
    Some(rho.min(1.0))
}

/// Fixed-point solver for ψ(z) = M z + t constrained to a strict polygon C, minimizing A.
pub fn fixed_point_in_poly(
    psi: Aff2,
    c: &Poly2,
    a: &Aff1,
    cfg: GeomCfg,
) -> Option<(Vector2<f64>, f64)> {
    let mat = Matrix2::identity() - psi.m;
    let svd = SVD::new(mat, true, true);
    let u = svd.u?;
    let vt = svd.v_t?;
    let v = vt.transpose();
    let s = svd.singular_values;
    let mut rank = 0;
    for i in 0..2 {
        if s[i] > cfg.eps_det {
            rank += 1;
        }
    }
    let ut_t = u.transpose() * psi.t;
    match rank {
        2 => {
            let z = mat.try_inverse()? * psi.t;
            if c.contains_eps(z, cfg.eps_feas) {
                Some((z, a.eval(z)))
            } else {
                None
            }
        }
        1 => {
            let zero_idx = if s[0] <= cfg.eps_det { 0 } else { 1 };
            if ut_t[zero_idx].abs() > cfg.eps_tau {
                return None;
            }
            let nz_idx = 1 - zero_idx;
            let coef = ut_t[nz_idx] / s[nz_idx];
            let z_part = v.column(nz_idx) * coef;
            let z_part = Vector2::new(z_part[0], z_part[1]);
            let dir_col = v.column(zero_idx);
            let dir = Vector2::new(dir_col[0], dir_col[1]);

            // Clip line segment by half-spaces
            let mut alpha_lo = f64::NEG_INFINITY;
            let mut alpha_hi = f64::INFINITY;
            for h in &c.hs {
                let nd = h.n.dot(&dir);
                let rhs = h.c - h.n.dot(&z_part);
                if nd.abs() <= cfg.eps_det {
                    if rhs < -cfg.eps_feas {
                        return None;
                    }
                } else if nd > 0.0 {
                    alpha_hi = alpha_hi.min(rhs / nd);
                } else {
                    alpha_lo = alpha_lo.max(rhs / nd);
                }
                if alpha_lo > alpha_hi + cfg.eps_feas {
                    return None;
                }
            }
            let slope = a.a.dot(&dir);
            let mut alpha = 0.0;
            if slope > cfg.eps_tau {
                alpha = alpha_lo;
            } else if slope < -cfg.eps_tau {
                alpha = alpha_hi;
            } else {
                if alpha < alpha_lo {
                    alpha = alpha_lo;
                }
                if alpha > alpha_hi {
                    alpha = alpha_hi;
                }
            }
            if alpha < alpha_lo {
                alpha = alpha_lo;
            }
            if alpha > alpha_hi {
                alpha = alpha_hi;
            }
            let z = z_part + dir * alpha;
            if c.contains_eps(z, cfg.eps_feas) {
                Some((z, a.eval(z)))
            } else {
                None
            }
        }
        _ => {
            if psi.t.norm() > cfg.eps_tau {
                None
            } else {
                match c.halfspace_intersection() {
                    HalfspaceIntersection::Bounded(verts) => {
                        let mut best: Option<(Vector2<f64>, f64)> = None;
                        for z in verts {
                            let val = a.eval(z);
                            if best.as_ref().is_none_or(|(_, v)| val < *v) {
                                best = Some((z, val));
                            }
                        }
                        best
                    }
                    _ => None,
                }
            }
        }
    }
}
