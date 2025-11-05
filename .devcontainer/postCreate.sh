#!/usr/bin/env bash
set -euo pipefail

# Persistent state under workspace .persist, symlinked into expected locations
# SET UP SYMLINKS FIRST before any tool generates caches/config
PERSIST_DIR="${WORKSPACE_FOLDER:-$PWD}/.persist"
mkdir -p "$PERSIST_DIR"

# Caches: npm, uv, pip, ruff
mkdir -p "$HOME/.cache"
mkdir -p "$PERSIST_DIR/npm" "$PERSIST_DIR/npm-cache"
ln -sfn "$PERSIST_DIR/npm" "$HOME/.npm"
ln -sfn "$PERSIST_DIR/npm-cache" "$HOME/.cache/npm"
mkdir -p "$PERSIST_DIR/uv-cache"
ln -sfn "$PERSIST_DIR/uv-cache" "$HOME/.cache/uv"
mkdir -p "$PERSIST_DIR/pip-cache"
ln -sfn "$PERSIST_DIR/pip-cache" "$HOME/.cache/pip"
mkdir -p "$PERSIST_DIR/ruff-cache"
ln -sfn "$PERSIST_DIR/ruff-cache" "$HOME/.cache/ruff"

# Codex CLI config
mkdir -p "$PERSIST_DIR/codex"
rm -rf "$HOME/.codex" 2>/dev/null || true
ln -sfn "$PERSIST_DIR/codex" "$HOME/.codex"

# Vibe Kanban local state
mkdir -p "$PERSIST_DIR/vibe-kanban" "$HOME/.local/share"
rm -rf "$HOME/.local/share/vibe-kanban" 2>/dev/null || true
ln -sfn "$PERSIST_DIR/vibe-kanban" "$HOME/.local/share/vibe-kanban"

# Vibe Kanban worktrees under /var/tmp
mkdir -p "$PERSIST_DIR/vibe-worktrees"
sudo mkdir -p /var/tmp/vibe-kanban
sudo rm -rf /var/tmp/vibe-kanban/worktrees 2>/dev/null || true
sudo ln -sfn "$PERSIST_DIR/vibe-worktrees" /var/tmp/vibe-kanban/worktrees
sudo chown -R "$(id -u):$(id -g)" "$PERSIST_DIR/vibe-worktrees"

# GitHub CLI config
mkdir -p "$PERSIST_DIR/gh-config" "$HOME/.config"
rm -rf "$HOME/.config/gh" 2>/dev/null || true
ln -sfn "$PERSIST_DIR/gh-config" "$HOME/.config/gh"

# Bash history (file)
mkdir -p "$PERSIST_DIR"
touch "$PERSIST_DIR/bash_history"
ln -sfn "$PERSIST_DIR/bash_history" "$HOME/.bash_history"

# Now run installs after symlinks are in place
# Ensure Rust cargo/rustup are in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Install additional Rust tools
cargo install cargo-nextest cargo-audit cargo-deny --locked || true
cargo install mdbook --locked || true

# Install Codex CLI
npm i -g @openai/codex || true

# Verify tools are available
node -v || true
npm -v || true
npx -v || true
gh --version || true
rg --version || true
uv --version || true
uvx --version || true
