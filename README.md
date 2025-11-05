# rust-viterbo

Agent-first MSc thesis repo. Source of truth: VK tickets → thesis spec (mdBook) → code/tests.
Thesis (working title): “Probing Viterbo's Conjecture”.

- Docs: `docs/` (build with `mdbook build docs`)
- Library: `crates/viterbo` (Rust lib, `nalgebra`)
- CLI: `crates/cli` (Rust bin, `clap`, `polars`)
- Outputs: heavy `data/` (gitignored), published `docs/assets/`
- Dev: `.devcontainer/`
- Scripts: `scripts/` (manual CI, safe exec, VK, checks)

License: MIT (see `LICENSE`).

Run local CI:
```

bash scripts/ci.sh

```
