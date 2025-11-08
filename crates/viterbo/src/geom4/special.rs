//! Special 4D polytopes used in tests and benchmarks.
//!
//! Purpose
//! - Provide canonical H-representations for common families:
//!   hypercubes, cross polytopes (ℓ1 balls), and orthogonal simplices.
//! - Keep constructors small and explicit so tests can rely on them deterministically.
//!
//! References
//! - Volume formulas:
//!   - Hypercube [-a,a]^4: vol = (2a)^4.
//!   - Cross polytope {‖x‖₁ ≤ r} in R⁴: vol = 2⁴ r⁴ / 4! = (2/3) r⁴.
//!   - Right (orthogonal) simplex with edge lengths (a,b,c,d):
//!     vol = a·b·c·d / 4! (via det of edge matrix).

use nalgebra::Vector4;

use super::types::{Hs4, Poly4};

/// Axis-aligned hypercube [-a,a]^4.
pub fn hypercube(a: f64) -> Poly4 {
    let mut hs = Vec::with_capacity(8);
    hs.push(Hs4::new(Vector4::new(1.0, 0.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(-1.0, 0.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 1.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, -1.0, 0.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 1.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, -1.0, 0.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 0.0, 1.0), a));
    hs.push(Hs4::new(Vector4::new(0.0, 0.0, 0.0, -1.0), a));
    Poly4::from_h(hs)
}

/// 4D cross polytope (ℓ1 ball): {x ∈ R⁴ : |x₁|+|x₂|+|x₃|+|x₄| ≤ r}.
pub fn cross_polytope_l1(r: f64) -> Poly4 {
    let mut hs: Vec<Hs4> = Vec::with_capacity(16);
    // All sign patterns s ∈ {±1}^4 define s·x ≤ r.
    for &sx in &[-1.0, 1.0] {
        for &sy in &[-1.0, 1.0] {
            for &sz in &[-1.0, 1.0] {
                for &sw in &[-1.0, 1.0] {
                    let n = Vector4::new(sx, sy, sz, sw);
                    hs.push(Hs4::new(n, r));
                }
            }
        }
    }
    Poly4::from_h(hs)
}

/// Right (orthogonal) 4D simplex (orthoscheme) with edge lengths (a,b,c,d).
///
/// Construction:
/// - v0 = 0
/// - v1 = a e1
/// - v2 = v1 + b e2
/// - v3 = v2 + c e3
/// - v4 = v3 + d e4
/// Then translate by the centroid so that 0 lies in the interior.
pub fn orthogonal_simplex(a: f64, b: f64, c: f64, d: f64) -> Poly4 {
    let v0 = Vector4::new(0.0, 0.0, 0.0, 0.0);
    let v1 = Vector4::new(a, 0.0, 0.0, 0.0);
    let v2 = Vector4::new(a, b, 0.0, 0.0);
    let v3 = Vector4::new(a, b, c, 0.0);
    let v4 = Vector4::new(a, b, c, d);
    let centroid = (v0 + v1 + v2 + v3 + v4) / 5.0;
    let verts = vec![v0 - centroid, v1 - centroid, v2 - centroid, v3 - centroid, v4 - centroid];
    Poly4::from_v(verts)
}

