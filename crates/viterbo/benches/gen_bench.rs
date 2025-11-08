//! Criterion microbenches for 2D/4D generators and hot-path combinators.
//!
//! - 2D: radial sampler, recenter/rescale, polar.
//! - 4D: random vertices (5–25),
//!   random faces (5–10),
//!   symmetric halfspaces,
//!   Mahler next/regen,
//!   regular product enum.
//!
//! Results live under `target/criterion`. Use `scripts/rust-bench.sh` to sync curated
//! JSON into `data/bench/criterion` (Git LFS) when needed.
//!
//! Ticket: 8ed3-2d-4d-generators

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use nalgebra::Matrix4;
use rand::{rngs::StdRng, Rng, SeedableRng};
use viterbo::geom2::rand::{
    draw_polygon_radial, polar as polar2, recenter_rescale, Bounds2, RadialCfg, ReplayToken,
    VertexCount,
};
use viterbo::rand4::{
    MahlerProductGenerator, MahlerProductParams, PolytopeGenerator4, RandomFacesGenerator,
    RandomFacesParams, RandomVerticesGenerator, RandomVerticesParams, RegularPolygonSpec,
    RegularProductEnumParams, RegularProductEnumerator, SymmetricHalfspaceGenerator,
    SymmetricHalfspaceParams,
};

fn bench_gen_2d(c: &mut Criterion) {
    let mut group = c.benchmark_group("gen2d");
    let cfg = RadialCfg {
        vertex_count: VertexCount::Uniform { min: 6, max: 12 },
        angle_jitter_frac: 0.25,
        radial_jitter: 0.2,
        base_radius: 1.0,
        random_phase: true,
    };
    let bounds = Bounds2 {
        r_in_min: 0.2,
        r_out_max: 2.0,
    };
    group.bench_function(BenchmarkId::new("draw_polygon_radial", "6-12"), |b| {
        b.iter_batched(
            || ReplayToken { seed: 42, index: 0 },
            |mut tok| {
                tok.index = tok.index.wrapping_add(1);
                let _ = draw_polygon_radial(cfg, tok);
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function(BenchmarkId::new("recenter_rescale", "bounds"), |b| {
        b.iter_batched(
            || {
                let tok = ReplayToken { seed: 7, index: 99 };
                draw_polygon_radial(cfg, tok).unwrap()
            },
            |p| {
                let _ = recenter_rescale(&p, bounds);
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function(BenchmarkId::new("polar", "post-center"), |b| {
        b.iter_batched(
            || {
                let tok = ReplayToken { seed: 9, index: 5 };
                let p = draw_polygon_radial(cfg, tok).unwrap();
                recenter_rescale(&p, bounds).unwrap().0
            },
            |p_centered| {
                let _ = polar2(&p_centered);
            },
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

fn bench_gen_4d(c: &mut Criterion) {
    let mut group = c.benchmark_group("gen4d");

    // Random vertices (V→H)
    let rv = RandomVerticesParams {
        vertices_min: 5,
        vertices_max: 25,
        radius_min: 0.4,
        radius_max: 1.2,
        anisotropy: None,
        max_attempts: 10,
    };
    group.bench_function(BenchmarkId::new("random_vertices_next", "5-25"), |b| {
        b.iter_batched(
            || RandomVerticesGenerator::new(rv.clone(), 11).unwrap(),
            |mut gen| {
                let _ = gen.generate_next().unwrap().unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    // Random faces (H→V→H reduction)
    let rf = RandomFacesParams {
        facets_min: 5,
        facets_max: 10,
        radius_min: 0.4,
        radius_max: 1.2,
        anisotropy: None,
        max_attempts: 20,
    };
    group.bench_function(BenchmarkId::new("random_faces_next", "5-10"), |b| {
        b.iter_batched(
            || RandomFacesGenerator::new(rf.clone(), 22).unwrap(),
            |mut gen| {
                let _ = gen.generate_next().unwrap().unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    // Symmetric halfspaces (even facets; use d=5 → 10 facets)
    let shp = SymmetricHalfspaceParams {
        directions: 5,
        radius_min: 0.2,
        radius_max: 1.0,
        anisotropy: Some(Matrix4::new(
            1.1, 0.0, 0.0, 0.0, 0.0, 0.9, 0.0, 0.0, 0.0, 0.0, 1.05, 0.0, 0.0, 0.0, 0.0, 0.95,
        )),
    };
    group.bench_function(
        BenchmarkId::new("sym_halfspaces_generate_single", "d5"),
        |b| {
            b.iter_batched(
                || StdRng::seed_from_u64(123).gen::<u64>(),
                |seed| {
                    let _ = SymmetricHalfspaceGenerator::generate_single(&shp, seed).unwrap();
                },
                BatchSize::SmallInput,
            )
        },
    );

    // Mahler product next + regen
    let mp = MahlerProductParams::default();
    group.bench_function(BenchmarkId::new("mahler_next", "default"), |b| {
        b.iter_batched(
            || MahlerProductGenerator::new(mp.clone(), 2025).unwrap(),
            |mut gen| {
                let _ = gen.generate_next().unwrap().unwrap();
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function(BenchmarkId::new("mahler_regen", "default"), |b| {
        b.iter_batched(
            || {
                let mut gen = MahlerProductGenerator::new(mp.clone(), 2025).unwrap();
                let s = gen.generate_next().unwrap().unwrap();
                (gen, s.replay)
            },
            |(gen, replay)| {
                let _ = gen.regenerate(&replay).unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    // Regular product enumerator
    let a = RegularPolygonSpec::new(8, 0.0, 1.0).unwrap();
    let b = RegularPolygonSpec::new(10, 0.2, 0.9).unwrap();
    let params = RegularProductEnumParams {
        factors_a: vec![a],
        factors_b: vec![b],
        max_pairs: Some(1),
    };
    group.bench_function(BenchmarkId::new("regular_product_next", "8x10"), |b| {
        b.iter_batched(
            || RegularProductEnumerator::new(params.clone()).unwrap(),
            |mut gen| {
                let _ = gen.generate_next().unwrap().unwrap();
            },
            BatchSize::SmallInput,
        )
    });
    group.bench_function(BenchmarkId::new("regular_product_regen", "8x10"), |b| {
        b.iter_batched(
            || {
                let mut gen = RegularProductEnumerator::new(params.clone()).unwrap();
                let s = gen.generate_next().unwrap().unwrap();
                (gen, s.replay)
            },
            |(gen, replay)| {
                let _ = gen.regenerate(&replay).unwrap();
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

criterion_group!(benches, bench_gen_2d, bench_gen_4d);
criterion_main!(benches);
