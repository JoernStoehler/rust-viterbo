//! Criterion microbenches for oriented‑edge internals (group "oe4").
//!
//! - ψ_ij push‑forward of strict H‑polys.
//! - τ‑inequality assembly/evaluation on a facet.
//! - Per‑edge lower‑bound computation over HPI vertices.
//!
//! These benches use a small, deterministic 4D cube and sample a subset of edges
//! from the constructed graph to keep runs fast and stable.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use nalgebra::Vector4;
use viterbo::api::*;
use viterbo::oriented_edge::Ridge;
use viterbo::prelude::HalfspaceIntersection;

fn cube4(side: f64) -> Poly4 {
    let s = side;
    let axes = [
        Vector4::new(1.0, 0.0, 0.0, 0.0),
        Vector4::new(0.0, 1.0, 0.0, 0.0),
        Vector4::new(0.0, 0.0, 1.0, 0.0),
        Vector4::new(0.0, 0.0, 0.0, 1.0),
    ];
    let mut hs = Vec::new();
    for a in axes {
        hs.push(Hs4::new(a, s));
        hs.push(Hs4::new(-a, s));
    }
    Poly4::from_h(hs)
}

fn bench_push_forward(c: &mut Criterion) {
    let mut group = c.benchmark_group("oe4");
    group.throughput(Throughput::Elements(100));
    let cfg = GeomCfg::default();
    let mut p4 = cube4(1.0);
    let g = build_graph(&mut p4, cfg);
    // Take up to N edges with bounded domains for stability.
    let edges: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            // Some edges (especially on symmetric cubes) produce singular
            // ψ_ij maps when the ridge is parallel to the Reeb direction.
            // Skip them so the bench keeps a stable workset instead of
            // panicking on `.push_forward` expecting invertibility.
            e.map_ij.m.determinant().abs() > 1e-12
                && matches!(
                    e.dom_in.halfspace_intersection(),
                    HalfspaceIntersection::Bounded(_)
                )
        })
        .take(64)
        .cloned()
        .collect();
    group.bench_function("psi_push_forward", |b| {
        b.iter_batched(
            || edges.clone(),
            |batch| {
                let mut acc = 0usize;
                for e in &batch {
                    let im = e.dom_in.push_forward(&e.map_ij).expect("invertible");
                    // Simple use to avoid optimizing away
                    acc = acc.wrapping_add(im.hs.len());
                }
                acc
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_tau_inequalities(c: &mut Criterion) {
    let mut group = c.benchmark_group("oe4");
    let cfg = GeomCfg::default();
    let mut p4 = cube4(1.0);
    let hs = p4.h.clone();
    let v_f = reeb_on_facets(&hs);
    let g = build_graph(&mut p4, cfg);
    // Build by‑facet index for quick scans.
    let mut by_facet: Vec<Vec<usize>> = vec![Vec::new(); g.num_facets];
    for (rid, r) in g.ridges.iter().enumerate() {
        by_facet[r.facets.0 .0].push(rid);
        by_facet[r.facets.1 .0].push(rid);
    }
    let other_facet = |r: &Ridge, f: usize| -> usize {
        let (a, b) = (r.facets.0 .0, r.facets.1 .0);
        if a == f {
            b
        } else {
            a
        }
    };
    // Collect small workset: (ri, rj, facet, chart_ut) and precomputed v
    let mut work = Vec::new();
    for e in g.edges.iter().take(128) {
        let ri = &g.ridges[e.from.0];
        let rj = &g.ridges[e.to.0];
        let f = e.facet.0;
        let v = v_f[f];
        let hj = other_facet(rj, f);
        let dj = hs[hj].n.dot(&v);
        if dj > cfg.eps_tau {
            work.push((e.from.0, e.to.0, f, ri.chart_ut, v));
        }
    }
    group.throughput(Throughput::Elements(work.len() as u64));
    group.bench_function("tau_inequality_eval", |b| {
        b.iter(|| {
            let mut count = 0usize;
            for (i_idx, j_idx, f, ut_i, v) in &work {
                let ri = &g.ridges[*i_idx];
                let rj = &g.ridges[*j_idx];
                let hj = other_facet(rj, *f);
                let dj = hs[hj].n.dot(v);
                if let HalfspaceIntersection::Bounded(verts) = ri.poly.halfspace_intersection() {
                    for y in &verts {
                        let x = ut_i * *y;
                        let num_j = hs[hj].c - hs[hj].n.dot(&x);
                        for &rk in &by_facet[*f] {
                            let hk = other_facet(&g.ridges[rk], *f);
                            if hk == hj {
                                continue;
                            }
                            let dk = hs[hk].n.dot(v);
                            if dk <= cfg.eps_tau {
                                continue;
                            }
                            let num_k = hs[hk].c - hs[hk].n.dot(&x);
                            let _ok = dk * num_j <= dj * num_k + 1e-9;
                            count = count.wrapping_add(_ok as usize);
                        }
                    }
                }
            }
            count
        })
    });
    group.finish();
}

fn bench_per_edge_lower_bound(c: &mut Criterion) {
    let mut group = c.benchmark_group("oe4");
    let cfg = GeomCfg::default();
    let mut p4 = cube4(1.0);
    let g = build_graph(&mut p4, cfg);
    let edges: Vec<_> = g
        .edges
        .iter()
        .filter(|e| {
            matches!(
                e.dom_in.halfspace_intersection(),
                HalfspaceIntersection::Bounded(_)
            )
        })
        .take(64)
        .cloned()
        .collect();
    group.throughput(Throughput::Elements(edges.len() as u64));
    group.bench_function("edge_lb_action", |b| {
        b.iter(|| {
            let mut acc = 0f64;
            for e in &edges {
                let lb = match e.dom_in.halfspace_intersection() {
                    HalfspaceIntersection::Bounded(verts) => verts
                        .into_iter()
                        .map(|z| e.action_inc.eval(z))
                        .fold(f64::INFINITY, f64::min),
                    _ => f64::NEG_INFINITY,
                };
                acc += lb;
            }
            acc
        })
    });
    group.finish();
}

fn oe4_benches(c: &mut Criterion) {
    bench_push_forward(c);
    bench_tau_inequalities(c);
    bench_per_edge_lower_bound(c);
}

criterion_group!(benches, oe4_benches);
criterion_main!(benches);
