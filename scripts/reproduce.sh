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
OUT_PROV=""
FIG_PROV=""

provenance_path() {
  local artifact="$1"
  local dir base stem
  dir="$(dirname "$artifact")"
  base="$(basename "$artifact")"
  stem="${base%.*}"
  if [[ -z "$stem" || "$stem" == "$base" ]]; then
    stem="${base:-artifact}"
  fi
  printf "%s/%s.provenance.json" "$dir" "$stem"
}

OUT_PROV="$(provenance_path "$OUT")"
FIG_PROV="$(provenance_path "$FIG")"

echo "🔧 Building workspace (cargo build --workspace)"
if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ Rust toolchain not found. Please use GitHub Codespaces or VS Code Dev Container, or install Rust." >&2
  exit 1
fi
bash scripts/safe.sh --timeout 900 -- cargo build --workspace

echo "🧾 Provenance report"
bash scripts/safe.sh --timeout 60 -- cargo run -p cli -- report || true

echo "▶️  Demo run → $OUT"
bash scripts/safe.sh --timeout 180 -- cargo run -p cli -- run --algo demo --input "$INPUT" --out "$OUT"

echo "🖼  Demo figure → $FIG"
bash scripts/safe.sh --timeout 180 -- cargo run -p cli -- figure --from "$OUT" --out "$FIG"

echo "📄  Syncing bibliography papers → data/downloads"
bash scripts/paper-download.sh --all

echo "📚 Rebuilding book for offline viewing (mdbook build docs)"
bash scripts/safe.sh --timeout 600 -- mdbook build docs

echo "✅ Reproduction complete. Outputs:"
echo "   - Heavy: $OUT and $OUT_PROV"
echo "   - Small: $FIG and $FIG_PROV"
echo "   - Thesis (offline): docs/book/index.html (if mdbook was available)"
