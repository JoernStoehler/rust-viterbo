//! Criterion benchmarks for the 4D volume algorithm.
//!
//! Runs the facet-fan volume computation on randomly generated H-reps with
//! varying numbers of facets to capture scaling behavior.

use criterion::{black_box, criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use nalgebra::Vector4;
use rand::{rngs::StdRng, Rng, SeedableRng};
use viterbo::geom4::{volume4, Hs4, Poly4};

fn random_polytope(facets: usize, seed: u64) -> Poly4 {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut hs = Vec::with_capacity(facets + 8);
    // Start from a cube [-1,1]^4 to guarantee boundedness.
    for axis in 0..4 {
        let mut pos = Vector4::zeros();
        pos[axis] = 1.0;
        hs.push(Hs4::new(pos, 1.0));
        let mut neg = Vector4::zeros();
        neg[axis] = -1.0;
        hs.push(Hs4::new(neg, 1.0));
    }
    // Add random outward half-spaces; keep the origin inside by forcing c>0.
    for _ in 0..facets {
        let mut n = Vector4::new(
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
            rng.gen_range(-1.0..1.0),
        );
        let mut norm = n.norm();
        while norm < 1e-6 {
            n = Vector4::new(
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
                rng.gen_range(-1.0..1.0),
            );
            norm = n.norm();
        }
        n /= norm;
        let c = rng.gen_range(0.6..1.8);
        hs.push(Hs4::new(n, c));
    }
    Poly4::from_h(hs)
}

fn bench_volume4(c: &mut Criterion) {
    let mut group = c.benchmark_group("geom4_volume");
    for &facets in &[12usize, 24, 48, 72] {
        group.bench_with_input(BenchmarkId::from_parameter(facets), &facets, |b, &m| {
            b.iter_batched(
                || random_polytope(m, 123 + m as u64),
                |mut poly| {
                    let _ = black_box(volume4(&mut poly).unwrap());
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(benches, bench_volume4);
criterion_main!(benches);
