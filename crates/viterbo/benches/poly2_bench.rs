//! Criterion benchmarks for 2D H-rep polytopes.
//! Focus sizes: m in {0, 10, 20, 50, 100}.
//! Results: by default under target/criterion; to store under data/bench, run:
//!   CARGO_TARGET_DIR=data/bench cargo bench -p viterbo

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use nalgebra::Vector2;
use rand::{rngs::StdRng, Rng, SeedableRng};
use viterbo::geom2::{Aff2 as Affine2, Hs2, Poly2 as HPoly2Ordered};

fn random_halfspaces(m: usize, seed: u64) -> HPoly2Ordered {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut hs = Vec::with_capacity(m);
    for _ in 0..m {
        // random angle and distance so that origin is inside most of the time
        let theta: f64 = rng.gen::<f64>() * std::f64::consts::TAU;
        let n = Vector2::new(theta.cos(), theta.sin());
        let c = rng.gen_range(0.5..1.5);
        hs.push(Hs2::new(n, c));
    }
    let mut ordered = HPoly2Ordered::default();
    for h in hs {
        ordered.insert_halfspace(h);
    }
    ordered
}

fn bench_poly2(c: &mut Criterion) {
    let mut group = c.benchmark_group("poly2");
    for &m in &[0usize, 10, 20, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("halfspace_intersection", m),
            &m,
            |b, &m| {
                b.iter_batched(
                    || random_halfspaces(m, 43),
                    |po| {
                        let _res = po.halfspace_intersection();
                    },
                    BatchSize::SmallInput,
                )
            },
        );

        group.bench_with_input(BenchmarkId::new("push_forward_strict", m), &m, |b, &m| {
            let f = Affine2 {
                m: nalgebra::matrix![1.2, 0.1; -0.05, 0.9],
                t: Vector2::new(0.3, -0.2),
            };
            b.iter_batched(
                || random_halfspaces(m, 44),
                |po| {
                    let _p2 = po.push_forward(&f).unwrap();
                },
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_poly2);
criterion_main!(benches);
