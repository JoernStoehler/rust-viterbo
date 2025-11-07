#!/usr/bin/env bash
# rust-bench.sh — cargo bench convenience wrapper (requires safe.sh)
# Contract
# - Must be invoked under scripts/safe.sh (checks SAFE_WRAPPED=1).
# - No internal timeouts; inherits the top-level timeout from safe.sh.
# - Defaults cargo target dir to the workspace target/ (Cargo default). Criterion output is
#   rsynced into data/bench after the run so Git LFS only tracks the JSON that matters.
# - BENCH_EXPORT_DIR controls where curated artifacts land (default: data/bench).
# - BENCH_RUN_POSTPROCESS=1 enables running the Python bench stage after export (default: 0 — run it explicitly via python -m viterbo....).
# - BENCH_EXPORT_RESULTS=0 skips the rsync copy (default: 1).
# Usage:
#   safe -t 300 -- bash scripts/rust-bench.sh [-p viterbo] [-- <extra cargo bench args>]
# Examples:
#   safe -t 300 -- bash scripts/rust-bench.sh
#   safe -t 120 -- bash scripts/rust-bench.sh -- --no-run         # compile benches only
#   BENCH_RUN_POSTPROCESS=1 safe -t 300 -- bash scripts/rust-bench.sh      # auto-run docs stage
set -euo pipefail

if [[ "${SAFE_WRAPPED:-}" != "1" ]]; then
  echo "error: scripts/rust-bench.sh must be run under scripts/safe.sh (global timeout). See AGENTS.md → Command Line Quick Reference." >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

PKG="viterbo"
EXTRA=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--package) PKG="$2"; shift 2 ;;
    --) shift; EXTRA+=("$@"); break ;;
    *) EXTRA+=("$1"); shift ;;
  esac
done

# Default cargo target dir to the workspace target/ unless overridden.
DEFAULT_TARGET_DIR="$ROOT_DIR/target"
TARGET_DIR="${CARGO_TARGET_DIR:-$DEFAULT_TARGET_DIR}"
if [[ "$TARGET_DIR" != /* ]]; then
  TARGET_DIR="$ROOT_DIR/$TARGET_DIR"
fi
export CARGO_TARGET_DIR="$TARGET_DIR"
mkdir -p "$CARGO_TARGET_DIR"

EXPORT_ROOT="${BENCH_EXPORT_DIR:-$ROOT_DIR/data/bench}"
if [[ "$EXPORT_ROOT" != /* ]]; then
  EXPORT_ROOT="$ROOT_DIR/$EXPORT_ROOT"
fi
EXPORT_CRITERION="$EXPORT_ROOT/criterion"
RUN_POSTPROCESS="${BENCH_RUN_POSTPROCESS:-0}"
COPY_RESULTS="${BENCH_EXPORT_RESULTS:-1}"
STAGE_CONFIG="${BENCH_STAGE_CONFIG:-$ROOT_DIR/configs/bench/docs_local.json}"
if [[ "$STAGE_CONFIG" != /* ]]; then
  STAGE_CONFIG="$ROOT_DIR/$STAGE_CONFIG"
fi
ASSETS_ROOT="${BENCH_ASSETS_DIR:-$ROOT_DIR/docs/assets/bench}"
if [[ "$ASSETS_ROOT" != /* ]]; then
  ASSETS_ROOT="$ROOT_DIR/$ASSETS_ROOT"
fi

echo ">>> cargo bench (-p $PKG) target=$CARGO_TARGET_DIR ${EXTRA[*]:-}"
cargo bench -p "$PKG" "${EXTRA[@]:-}"
echo "Rust benches completed."

if [[ "$COPY_RESULTS" == "1" ]] && [[ -d "$CARGO_TARGET_DIR/criterion" ]]; then
  if ! command -v rsync >/dev/null 2>&1; then
    echo "error: rsync is required to export Criterion artifacts" >&2
    exit 1
  fi
  echo ">>> syncing Criterion artifacts to $EXPORT_CRITERION"
  mkdir -p "$EXPORT_CRITERION"
  rsync -a --delete "$CARGO_TARGET_DIR/criterion/" "$EXPORT_CRITERION/"
else
  echo ">>> skip Criterion sync (COPY_RESULTS=$COPY_RESULTS, dir=$CARGO_TARGET_DIR/criterion)"
fi

if [[ "$RUN_POSTPROCESS" == "1" ]] && [[ -d "$EXPORT_CRITERION" ]]; then
  echo ">>> post-processing Criterion artifacts via viterbo.bench.stage_docs"
  STAGE_ARGS=(--config "$STAGE_CONFIG" --bench-root "$EXPORT_CRITERION")
  if [[ -n "$ASSETS_ROOT" ]]; then
    STAGE_ARGS+=(--assets-root "$ASSETS_ROOT")
  fi
  if command -v uv >/dev/null 2>&1; then
    uv run python -m viterbo.bench.stage_docs "${STAGE_ARGS[@]}"
  else
    python3 -m viterbo.bench.stage_docs "${STAGE_ARGS[@]}"
  fi
else
  echo ">>> skip post-process (RUN_POSTPROCESS=$RUN_POSTPROCESS, dir=$EXPORT_CRITERION)"
fi
