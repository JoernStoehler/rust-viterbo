#!/usr/bin/env bash
set -euo pipefail

# Minimal, reader-friendly reproduction of the demo flow described in README.
# - Builds the workspace
# - Prints a provenance report
# - Runs a tiny demo pipeline (run → figure) with provenance sidecar files
# - Optionally rebuilds the book if mdbook is available (offline viewing)
#
# Usage:
#   bash scripts/reproduce.sh
#
# Environment variables (optional):
#   INPUT    Path to a small input file (default: docs/src/index.md)
#   OUT      Path to heavy output (default: data/demo/out.json)
#   FIG      Path to small figure (default: docs/assets/demo.json)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}" )"/.. && pwd)"
cd "$ROOT_DIR"

INPUT=${INPUT:-docs/src/index.md}
OUT=${OUT:-data/demo/out.json}
FIG=${FIG:-docs/assets/demo.json}

echo "🔧 Building workspace (cargo build --workspace)"
if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ Rust toolchain not found. Please use GitHub Codespaces or VS Code Dev Container, or install Rust." >&2
  exit 1
fi
cargo build --workspace

echo "🧾 Provenance report"
cargo run -p cli -- report || true

echo "▶️  Demo run → $OUT"
cargo run -p cli -- run --algo demo --input "$INPUT" --out "$OUT"

echo "🖼  Demo figure → $FIG"
cargo run -p cli -- figure --from "$OUT" --out "$FIG"

if command -v mdbook >/dev/null 2>&1; then
  echo "📚 Rebuilding book for offline viewing (mdbook build docs)"
  mdbook build docs
else
  echo "ℹ️  mdbook not found; skipping local book build (online reading recommended)."
fi

echo "✅ Reproduction complete. Outputs:"
echo "   - Heavy: $OUT and $(dirname "$OUT")/provenance.json"
echo "   - Small: $FIG and $(dirname "$FIG")/provenance.json"
echo "   - Thesis (offline): docs/book/index.html (if mdbook was available)"

