//! Tolerance defaults for 4D geometry (internal).
//!
//! Policy
//! - Defaults are fixed constants to avoid “tolerance juggling” during normal
//!   development. Adjustments are rare; if needed later we can make these
//!   configurable behind a small `Config` without changing call sites broadly.

/// Feasibility/memberhip epsilon used by `Hs4::satisfies` and geometric dedup.
pub(crate) const FEAS_EPS: f64 = 1e-9;
/// Tightness threshold for “near-active” inequalities during face enumeration.
pub(crate) const TIGHT_EPS: f64 = 1e-7;
/// Tolerance for symplectic check `M^T J M ≈ J` (max-abs metric).
pub(crate) const SYMPLECTIC_EPS: f64 = 1e-8;

