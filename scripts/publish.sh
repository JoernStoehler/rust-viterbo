#!/usr/bin/env bash
set -euo pipefail

# Publish docs/book to the gh-pages branch (manual, no CI).
#
# Usage:
#   bash scripts/publish.sh
#
# Notes:
# - Requires push access to the origin remote.
# - First run will create the gh-pages branch; then enable Pages in
#   GitHub Settings → Pages → Source: gh-pages, root.
# - Rebuilds the book and publishes the static files.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
BOOK_DIR="$ROOT_DIR/docs/book"

echo "🔧 Building the book..."
group-timeout 600 mdbook build "$ROOT_DIR/docs"

# Prepare a temporary worktree
TMP_DIR="$(mktemp -d)"
cleanup() { rm -rf "$TMP_DIR" || true; }
trap cleanup EXIT

echo "🌿 Preparing gh-pages worktree..."
if git show-ref --verify --quiet refs/heads/gh-pages; then
  git worktree add "$TMP_DIR" gh-pages
else
  git worktree add -b gh-pages "$TMP_DIR"
fi

echo "🧹 Clearing old contents..."
find "$TMP_DIR" -mindepth 1 -maxdepth 1 ! -name .git -exec rm -rf {} +

echo "📦 Copying new site..."
cp -r "$BOOK_DIR"/* "$TMP_DIR"/
touch "$TMP_DIR/.nojekyll"

echo "✅ Committing and pushing..."
pushd "$TMP_DIR" >/dev/null
git add -A
git commit -m "Publish docs: $(date -u +%F)" || echo "Nothing to commit."
git push -u origin gh-pages
popd >/dev/null

echo "🎉 Published. Configure GitHub Pages (once) to serve from gh-pages / root."
