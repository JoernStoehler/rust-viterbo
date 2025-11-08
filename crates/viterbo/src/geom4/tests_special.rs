//! Smoke tests for special 4D polytopes constructors.
//!
//! We only assert basic invariants (convexity, origin membership, facet counts)
//! to keep the suite robust while algorithms evolve.

use super::{special, Poly4};

#[test]
fn hypercube_basic_props() {
    let mut c = special::hypercube(1.0);
    assert!(c.contains_origin().unwrap_or(false));
    assert!(c.is_convex());
    assert_eq!(c.h.len(), 8);
}

#[test]
fn cross_polytope_basic_props() {
    let mut cp = special::cross_polytope_l1(1.0);
    assert!(cp.contains_origin().unwrap_or(false));
    assert!(cp.is_convex());
    assert_eq!(cp.h.len(), 16);
}

#[test]
fn orthogonal_simplex_basic_props() {
    // Convert to H-rep and check origin membership + convexity.
    let mut s = special::orthogonal_simplex(1.0, 1.0, 1.0, 1.0);
    s.ensure_halfspaces_from_v();
    assert!(s.contains_origin().unwrap_or(false));
    assert!(s.is_convex());
    // A 4-simplex has 5 facets; conversion may add coalesced planes, so check >= 5.
    assert!(s.h.len() >= 5);
}

