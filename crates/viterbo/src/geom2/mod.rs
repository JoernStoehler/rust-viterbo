//! Strict 2D Geometry (ordered H-representation only).
//!
//! Purpose
//! - Provide a single, strict, ordered H-rep polytope `Poly2` with
//!   unit-norm, angle-sorted, coalesced half-spaces for reliable, fast ops.
//! - Keep the API minimal (KISS, YAGNI) and numerically explicit (eps-aware).
//!
//! Why strict-only
//! - Aligns with the oriented-edge algorithm needs (push-forward, HPI, cuts).
//! - Avoids “loose→strict” conversions unless a hotspot is demonstrated later.
//!
//! References
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md
//! - AGENTS: AGENTS.md (Rust conventions, testing policy)
//! - Code cross-refs: `Poly2`, `Hs2`, `Aff2`, `Aff1`, `GeomCfg`

pub mod ordered;
pub mod rand;
mod solvers;
mod types;
mod util;

pub use ordered::{HalfspaceIntersection, Poly2};
pub use solvers::{fixed_point_in_poly, rotation_angle};
pub use types::{Aff1, Affine2 as Aff2, GeomCfg, Hs2};
pub use util::from_points_convex_hull_strict;

#[cfg(test)]
mod tests;
