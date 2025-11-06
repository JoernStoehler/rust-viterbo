#!/usr/bin/env bash
set -euo pipefail
bash scripts/safe.sh --timeout 900  -- cargo fmt --all -- --check
bash scripts/safe.sh --timeout 1200 -- cargo clippy --workspace --all-targets -- -D warnings
bash scripts/safe.sh --timeout 1800 -- cargo test --workspace
bash scripts/safe.sh --timeout 600  -- mdbook build docs
echo "CI OK"
