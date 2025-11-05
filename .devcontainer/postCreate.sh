#!/usr/bin/env bash
set -euo pipefail

# Persistent state under workspace .persist, symlinked into expected locations
# SET UP SYMLINKS FIRST before any tool generates caches/config
PERSIST_DIR="${WORKSPACE_FOLDER:-$PWD}/.persist"
mkdir -p "$PERSIST_DIR"

# Caches: rustup, cargo, sccache
mkdir -p "$PERSIST_DIR/cargo-home" "$PERSIST_DIR/rustup" "$PERSIST_DIR/sccache"
rm -rf "$HOME/.cargo" "$HOME/.rustup" 2>/dev/null || true
ln -sfn "$PERSIST_DIR/cargo-home" "$HOME/.cargo"
ln -sfn "$PERSIST_DIR/rustup" "$HOME/.rustup"
ln -sfn "$PERSIST_DIR/sccache" "$HOME/.sccache"

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
# Set envvars
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
 && rustup component add rustfmt clippy

# Install pre-built binaries (much faster than cargo install)
mkdir -p "$HOME/.cargo/bin"

# sccache
SCCACHE_VERSION="0.12.0"
curl -L "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-dist-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
  | tar -xz -C "$HOME/.cargo/bin" || true

# cargo-nextest
NEXTEST_VERSION="0.9.111"
curl -L "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
  | tar -xz -C "$HOME/.cargo/bin" cargo-nextest || true

# cargo-audit
AUDIT_VERSION="0.21.2"
curl -L "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v${AUDIT_VERSION}/cargo-audit-x86_64-unknown-linux-gnu.tar.gz" \
  | tar -xz -C "$HOME/.cargo/bin" || true

# cargo-deny
DENY_VERSION="0.18.5"
curl -L "https://github.com/EmbarkStudios/cargo-deny/releases/download/${DENY_VERSION}/cargo-deny-${DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" \
  | tar -xz -C "$HOME/.cargo/bin" cargo-deny || true

# mdbook
MDBOOK_VERSION="0.4.52"
curl -L "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/mdbook-v${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz" \
  | tar -xz -C "$HOME/.cargo/bin" mdbook || true

export RUSTC_WRAPPER="sccache"
export SCCACHE_DIR="$HOME/.sccache"

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
rustc --version || true
cargo --version || true
sccache --version || true

# Persist exports to .bashrc
cat >> "$HOME/.bashrc" << EOF
export PATH="\$HOME/.cargo/bin:\$PATH"
export CARGO_HOME="\$HOME/.cargo"
export RUSTUP_HOME="\$HOME/.rustup"
export RUSTC_WRAPPER="sccache"
export SCCACHE_DIR="\$HOME/.sccache"
EOF

echo "✅ Post-create setup completed successfully."