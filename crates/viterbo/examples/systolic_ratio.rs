//! Systolic ratio timing probe for a single 4D polytope.
//!
//! Purpose
//! - Provide a reproducible, code-backed data point for “how long does it take
//!   to evaluate c_EHZ and the systolic ratio on a ~9-facet polytope?”
//! - Feed the thesis status page with concrete timings instead of estimates.
//!
//! Why this shape
//! - We draw one `RandomFaces` sample with exactly nine facets so the half-space
//!   count matches the doc request.
//! - The generator already enforces boundedness and origin containment, so the
//!   oriented-edge algorithm can run unmodified.
//!
//! References
//! - TH: docs/src/thesis/status-math.md
//! - TH: docs/src/thesis/capacity-algorithm-oriented-edge-graph.md
//! - Code: crates/viterbo/src/oriented_edge/dfs.rs::solve_with_defaults

use std::time::Instant;

use nalgebra::Vector4;
use viterbo::geom4::{volume4, Hs4, Poly4};
use viterbo::oriented_edge::solve_with_defaults;

fn main() {
    let base = nine_facet_poly();
    let mut poly_for_capacity = base.clone();
    poly_for_capacity
        .check_canonical()
        .expect("polytope canonical");
    assert!(
        poly_for_capacity.h.len() == 9,
        "expected exactly nine facets, got {}",
        poly_for_capacity.h.len()
    );

    let cap_start = Instant::now();
    let (capacity, cycle) = solve_with_defaults(&mut poly_for_capacity)
        .expect("capacity solver should return a cycle");
    let cap_elapsed = cap_start.elapsed().as_secs_f64() * 1e3;

    let mut poly_for_volume = base.clone();
    let vol_start = Instant::now();
    let volume = volume4(&mut poly_for_volume).expect("volume succeeds");
    let vol_elapsed = vol_start.elapsed().as_secs_f64() * 1e3;

    let systolic = (capacity * capacity) / (2.0 * volume);

    println!(
        "family=cube_with_oblique_cap facets={} vertices={}",
        poly_for_capacity.h.len(),
        poly_for_capacity.v.len()
    );
    println!(
        "capacity={capacity:.9} systolic_ratio={systolic:.9} cycle_len={}",
        cycle.len()
    );
    println!("capacity_time_ms={cap_elapsed:.3}");
    println!("volume_time_ms={vol_elapsed:.3}");
}

fn nine_facet_poly() -> Poly4 {
    let axes = [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ];
    let mut hs = Vec::new();
    for axis in axes {
        hs.push(Hs4::new(axis, 1.0));
        hs.push(Hs4::new(-axis, 1.0));
    }
    let mut extra = Vector4::new(1.0, 1.0, 1.0, 1.0);
    extra /= extra.norm();
    hs.push(Hs4::new(extra, 1.8));
    let mut poly = Poly4::from_h(hs);
    poly.ensure_vertices_from_h();
    poly
}
