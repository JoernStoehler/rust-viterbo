<!-- Author: Codex & Jörn -->

# Atlas Dataset

Atlas aggregates many families of 4D, star-shaped, convex polytopes into one table so downstream experiments do not need to orchestrate generators manually. The `src/viterbo/atlas` package owns the pipeline:

- `stage_build.py` validates a JSON config and writes a Parquet dataset + provenance sidecar.
- `stage_visualize.py` turns any dataset into a compact JSON preview (`docs/assets/atlas/*.json`) that the mdBook can embed.
- `torch_dataset.py` exposes a minimal `torch.utils.data.Dataset` wrapper so ML experiments can pull features without bespoke glue.

Generator internals and mathematical context continue to live in [Random Polytope Generators](./random-polytopes.md#random-polytopes); this page documents how Atlas consumes them, the schema we keep stable, and the trade-offs we made.

## Row schema

Each row represents a single polytope. The columns we keep stable today are:

| Column            | Type            | Description |
| ----------------- | --------------- | ----------- |
| `row_id`          | int64           | Global monotonically increasing index. |
| `family`          | str             | Generator family name (`symmetric_halfspaces`, `mahler_products`, `regular_products`, `special_catalog`). |
| `family_name`     | str             | Human-readable alias from the config (e.g. `regular_demo` or `special_baseline:hypercube`). |
| `family_parameters` | JSON string  | Serialized copy of the generator parameters that produced the row (including derived seeds). |
| `replay_token`    | JSON string     | Token we pass back to the PyO3 bindings to regenerate the exact polytope. |
| `vertices`        | list\[list\[float\]] | V-representation, always eagerly materialized. |
| `halfspaces`      | list\[list\[float\]] | H-representation as `[n0, n1, n2, n3, c]` tuples. |
| `vertex_count` / `halfspace_count` | int64 | Derived counts, handy for quick slicing and for the preview asset. |
| `volume`          | float64         | Computed via `_native.poly4_volume_from_halfspaces`. |
| `capacity_ehz`    | float64         | Currently `NaN` (see “Gaps” below). |
| `dominant_orbit`  | str             | `"unavailable"` placeholder until we expose orbit finders. |
| `systolic_ratio`  | float64         | `capacity_ehz^2 / (2·volume)`; also `NaN` until capacities land. |

The row schema is intentionally redundant: we keep both H- and V-representations, plus replay metadata, so any downstream experiment can decide how lazy it wants to be.

## Source families

Each JSON source entry selects a family, number of rows, and family-specific parameters. The following mapping is implemented by `src/viterbo/atlas/sources.py`:

1. **`symmetric_halfspaces`** – repeatedly calls the PyO3 binding `rand4_symmetric_halfspace_sample(params, seed)` which wraps `SymmetricHalfspaceGenerator` from the Rust crate. Config knobs:
   - `directions`, `radius_min`, `radius_max`
   - optional `anisotropy` (4×4 matrix) to bias directions
2. **`mahler_products`** – deterministic sampling of Mahler products `K × K°`. Config carries the `radial_cfg` and `bounds` dictionaries described in the thesis.
3. **`regular_products`** – enumerates lagrangian products of two regular polygons. Config lists `factors_a`/`factors_b` (each `sides`, `rotation`, `scale`) plus `max_pairs`.
4. **`special_catalog`** – deterministic catalogue of hand-coded shapes (currently the hypercube `[-1,1]^4`, the cross polytope, and the orthogonal simplex). Config sets `rows` and a list of `members`; when `rows` exceeds the number of listed members we cycle the list.

Every random source derives its own stream seed from the global `config.seed`, plus an offset, so rows stay reproducible across versions.

## Config files

Config version 3 (see `configs/atlas/test.json`, `small.json`, `large.json`) uses the following structure:

```json
{
  "version": 3,
  "seed": 42,
  "sources": [
    {
      "name": "sym_tiny",
      "family": "symmetric_halfspaces",
      "rows": 4,
      "params": { "directions": 6, "radius_min": 0.7, "radius_max": 1.25 }
    },
    {
      "name": "mahler_probe",
      "family": "mahler_products",
      "rows": 3,
      "params": {
        "radial_cfg": { "vertex_count": {"kind": "uniform", "min": 6, "max": 8}, ... },
        "bounds": { "r_in_min": 0.2, "r_out_max": 2.5 },
        "max_attempts": 8
      }
    },
    { "... regular products ..." },
    { "... special catalog ..." }
  ],
  "out": {
    "dataset": "data/atlas/test.parquet",
    "preview": "docs/assets/atlas/test_preview.json",
    "preview_limit": 24
  }
}
```

Key points:

- Paths in `out` are relative to the repo root. The builder resolves them to absolute paths before writing.
- `rows` is mandatory except for catalogue sources where it can be inferred from the `members` list.
- `preview` is optional, but we keep it enabled for `test` and `small` so the mdBook always has a recent asset.
- `stage_build.py --preview-only --config <file>` lets us refresh the preview without regenerating the (possibly huge) dataset.

## Storage, previews, and alternatives

- **Storage format**: Apache Parquet with Zstd compression. Alternatives we considered:
  1. Arrow IPC/Feather – adds zero-copy sharing with Python, but Parquet keeps the datasets diffable and is friendlier for Git LFS.
  2. SQLite – tempting for random access but significantly more boilerplate, and harder to hook into Torch workflows.
  3. JSONL – extremely easy to inspect, but 10–100× larger and no nested-type schema.
- **Preview assets**: `docs/assets/atlas/*.json` contain a handful of high-signal columns (`row_id`, `family`, counts, `volume`, `systolic_ratio`). We intentionally drop the heavy geometry columns here so the mdBook can embed the table without exploding bundle size.
- **Torch loader**: `AtlasTorchDataset` (see `src/viterbo/atlas/torch_dataset.py`) takes a dataset path, list of feature columns, and optional target column, then exposes an iterable over `torch.float32` tensors. We keep the class tiny on purpose so experiments can subclass or wrap it as needed.

## Current gaps (and required Rust work)

- `capacity_ehz`, `dominant_orbit`, and the derived `systolic_ratio` are stubs today (`NaN` + `"unavailable"`). We need the following native helpers to finish the acceptance criteria:
  1. A PyO3 binding for whichever EHZ algorithm we settle on (likely the HK LP solver and the billiard code).
  2. A binding that returns not just the minimum action value but also the orbit description so we can populate `dominant_orbit`.
- Special catalogue currently ships hand-coded shapes. The Heim–Kislev counterexample and other literature polytopes still need to be coded up once the Rust side lands.
- Visualization is table-only. When we start comparing families we should add quick scatterplots (UMAP/t-SNE) once distance metrics become available.

## Companion files

- `configs/atlas/test.json` – tiny fixture used by the E2E test suite and docs preview.
- `configs/atlas/small.json` – ~10³ rows; runs in a few seconds and is the recommended day-to-day dataset for analysis.
- `configs/atlas/large.json` – aspirational 10⁶-row run (1–10 hours). We will not run this until the parallel EHZ implementations land.
- `docs/assets/atlas/test_preview.json` – committed preview (kept in sync by `stage_build`).
- `docs/assets/atlas/small_preview.json` – optional preview for the small dataset so the mdBook can demonstrate a larger slice.

Runbook snippet:

```bash
safe --timeout 60 -- python -m viterbo.atlas.stage_build --config configs/atlas/test.json
safe --timeout 60 -- python -m viterbo.atlas.stage_visualize --dataset data/atlas/test.parquet \
    --out docs/assets/atlas/test_preview.json --limit 32
```

That command sequence regenerates the dataset, provenance sidecar, and preview asset in one go.
