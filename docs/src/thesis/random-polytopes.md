# Random Polytope Generators {#random-polytopes}

<!-- Ticket: 0f48-random-polytopes -->
<!-- Code: crates/viterbo/src/rand4/mod.rs -->

Goal: define a plug-and-play catalogue of random (and enumerative) 4D polytopes that feed the atlas dataset and future experiments. Every generator must be reproducible, emit only valid polytopes, and surface enough metadata for downstream provenance sidecars.

## Generator Interface Conventions

- **Inputs** = `(params, seed)` for stochastic streams, or `(params, index)` for deterministic enumerations. `params` captures the distribution (facet counts, radii ranges, polygon choices, etc.), while the `seed/index` replay token pins down the first row produced by that generator.
- **Outputs** = structured rows: one coherent polytope representation (we standardize on `Poly4` with both H- and V-reps available) plus the replay token that regenerates the same row on demand.
- **Validity-first** = generators may internally sample/reject invalid candidates, but they must *yield only valid, star-shaped, origin-containing polytopes*. Throughput is secondary to correctness.
- **Enumerations** = allowed to stop after finitely many rows. They accept cutoff parameters (e.g., max number of facets) and may order outputs arbitrarily as long as replay tokens (`index` tuples) are stable.
- **Single vs stream APIs**:
  - `generate_single(params, seed)` returns one polytope and the canonical replay token.
  - `generate_stream(params, seed)` yields successive rows (possibly infinite). Streams expose a `next()` API but also allow replaying an individual row via the accompanying token without iterating the full stream.
- **Reproducibility** = every row stores the `params` snapshot and replay token next to the data artifact. When hydrating the dataset, we rebuild rows by calling `generate_single` with that information.

Implementation note: the `rand4` Rust module materializes these conventions via `GeneratorParams`, `ReplayToken`, `PolytopeSample4`, and the `PolytopeGenerator4` trait. Python orchestrators can call into PyO3 bindings once exposed.

## Algorithm Families

### 1. Centrally Symmetric Random Halfspaces
- **Idea**: sample `m` random directions on the 3-sphere (Gaussian → normalize) and add paired halfspaces `±n·x <= r`, where `r` is drawn from `[r_min, r_max]`.
- **Params**: number of directions, radius range, optional linear map to inject anisotropy.
- **Replay**: RNG seed. Replaying re-samples the identical sequence of normals/radii, so the first yielded polytope matches exactly.
- **Validity**: origin is always feasible (`0 <= r`). Paired halfspaces enforce boundedness; we reject configs where the linear map is singular.
- **Use cases**: broad “background” distribution for the atlas dataset; easy to tune between cubes (low variance) and rounded bodies (high `m`).

### 2. Mahler Product Sampler (2D × Polar)
- **Idea**: draw a random 2D convex polygon `K` (e.g., via radial samples), compute its polar `K^◦`, then form `K × K^◦ ⊂ ℝ⁴`. This family stays within the Mahler/Viterbo equivalence regime.
- **Params**: vertex count range for `K`, radial jitter budget, minimum/maximum in-radius to keep `K` full-dimensional.
- **Replay**: base seed + index mixed into the 2D sampler’s `ReplayToken`. Replaying regenerates the exact polygon, its polar, and their product.
- **Validity**: `K` contains the origin after `recenter_rescale`; the polar remains bounded. Cartesian products naturally yield star-shaped polytopes.
- **Implementation**: `rand4::MahlerProductGenerator` backed by `geom2::rand::{draw_polygon_radial, recenter_rescale, polar}`. Atlas stages can stream rows or rehydrate via the replay token.

### 3. Regular Polygon Product Enumerator
- **Idea**: enumerate tuples `(n₁, n₂, rotation₁, rotation₂, scale)` and build the lagrangian product of two regular `n`‑gons. Each tuple deterministically identifies a single 4D polytope.
- **Params**: discrete sets (or ranges) for `n_i`, rotation grids (e.g., multiples of `π/32`), and per-factor scales.
- **Replay**: the tuple itself. `generate_single` simply rebuilds the cartesian-product vertex set.
- **Validity**: direct product of convex polygons, so convex/stable automatically. Enumerations terminate once all tuples are exhausted or a cutoff is hit.

### 4. Perturbed Special Polytopes
- **Idea**: start from a catalog (cube, cross-polytope, Viterbo counterexample) and apply small randomized symplectic or affine perturbations. Useful for stress-testing capacity algorithms along known families.
- **Params**: base polytope id, perturbation budget, symplectic/affine toggle.
- **Replay**: `(base_id, seed)`; deterministic perturbation sequences.
- **Open question**: best way to constrain perturbations so that invariants (symplectic, lagrangian product structure) remain intact—escalate before implementing.

### 5. Streaming Filters / Rejection Pipelines
- **Idea**: wrap any generator with a predicate (e.g., “systolic ratio ≥ 0.9”) and expose a filtered stream. Inputs add a `filter_seed` for deterministic acceptance/rejection.
- **Policy**: the wrapped generator still owns the replay token; the filter stores “skip counts” so that regenerating row `k` repeats the same sequence of rejections before yielding.

## Integration with the Atlas Dataset

- **Row schema**: `{"polytope": Poly4, "generator": name, "params": json, "replay_token": value}`. The atlas build stage reads this schema to call `generate_single` when regenerating artifacts.
- **Config knobs**: each dataset config lists generators with explicit `rows` (or `max_rows` for enumerations). Scaling up/down means editing those integers directly, which keeps per-source cost controls obvious (e.g., “Mahler = 40 rows, Regular products = 10 rows, Catalog = 5 rows”).
- **Testing**: smoke configs cap each generator at ≤3 rows to keep `tests/smoke` under 10 seconds; full configs rely on `scripts/reproduce.sh`.
- **Escalation hooks**: if a generator cannot hit requested constraints (e.g., Mahler sampler fails to find a polygon with desired in-radius), it should emit structured errors referencing this page and the originating ticket.

## Next Steps

1. Surface the `rand4` module via PyO3 bindings so Python stages can stream polytopes without re-implementing algorithms.
2. Add benchmark hooks to record generation time per row (to correlate distributions with compute cost).
3. Expand the generator catalogue with sphere packings, zonotopes, and EHZ-focused adversarial shapes once the current families cover the baseline dataset needs.
