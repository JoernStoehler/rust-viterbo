#!/usr/bin/env bash
# rust-bench-quick.sh — faster Criterion run with lower warm-up/measurement (requires group-timeout)
# Intent: quick iteration signal, does not copy results to data/bench.
set -euo pipefail
SCRIPT_NAME="$(basename "${BASH_SOURCE[0]}")"
if [[ "${GROUP_TIMEOUT_ACTIVE:-}" != "1" ]]; then
  printf 'error: %s must be run under group-timeout (global timeout).\n' "$SCRIPT_NAME" >&2
  exit 2
fi
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
if command -v sccache >/dev/null 2>&1; then
  export RUSTC_WRAPPER="${RUSTC_WRAPPER:-sccache}"
fi
# Use a shared absolute target dir by default for cross-worktree reuse via sccache.
DEFAULT_TARGET_DIR="/workspaces/rust-viterbo/.persist/cargo-target"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$DEFAULT_TARGET_DIR}"
mkdir -p "$CARGO_TARGET_DIR"
PKG="viterbo"
# Quick preset derived from short sweeps (m=50):
# - Warm-up 0.5s is a good knee for “heated” steady state without long waits.
# - Measurement 2s + 40 samples yields low noise for local comparison while
#   keeping runs snappy. Use the regular preset for more stable publication numbers.
CRATE_MANIFEST="$ROOT_DIR/crates/$PKG/Cargo.toml"
BENCH_NAMES=()
if [[ -f "$CRATE_MANIFEST" ]]; then
  while IFS= read -r name; do
    [[ -n "$name" ]] && BENCH_NAMES+=("$name")
  done < <(awk '/^\[\[bench\]\]/{flag=1;next}/^\[/{flag=0}flag && /name *=/{gsub(/.*name *= *\"|\".*/,""); print}' "$CRATE_MANIFEST")
fi

if (( ${#BENCH_NAMES[@]} > 0 )); then
  echo ">>> running quick benches: ${BENCH_NAMES[*]}"
  for bname in "${BENCH_NAMES[@]}"; do
    echo ">>> cargo bench (-p $PKG --bench $bname) quick: warmup=0.5 measure=2 samples=40"
    cargo bench -p "$PKG" --bench "$bname" -- \
      --warm-up-time 0.5 \
      --measurement-time 2 \
      --sample-size 40
  done
else
  echo "warning: could not discover bench names; running cargo bench without per-bench args"
  cargo bench -p "$PKG"
fi
echo "Quick benches completed."
