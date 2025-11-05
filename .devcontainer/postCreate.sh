#!/usr/bin/env bash
set -euo pipefail

# WARNING: This file is super fragile, don't change ANYTHING unless you know what you're doing.

# Expect to run as the non-root dev user "vscode"
if [[ "$(id -un)" != "vscode" ]]; then
  echo "postCreate.sh should run as user 'vscode'." >&2
fi

# Persistent state under workspace .persist, symlinked into expected locations
# SET UP SYMLINKS FIRST before any tool generates caches/config
PERSIST_DIR="${WORKSPACE_FOLDER:-$PWD}/.persist"
mkdir -p "$PERSIST_DIR"

# Caches: rustup, cargo, sccache
mkdir -p "$PERSIST_DIR"/{cargo-home,rustup,sccache,codex,gh-config,vibe-kanban,npm,npm-cache,uv-cache,pip-cache,ruff-cache,vibe-worktrees}
touch "$PERSIST_DIR/bash_history"
rm -rf "$HOME"/{.cargo,.rustup,.sccache,.codex,.local/share/vibe-kanban,.config/gh,.bash_history,.npm,.cache/npm,.cache/uv,.cache/pip,.cache/ruff} 2>/dev/null || true
mkdir -p "$HOME/.local/bin" "$HOME/.local/share" "$HOME/.config"
ln -sfn "$PERSIST_DIR/cargo-home"   "$HOME/.cargo"
ln -sfn "$PERSIST_DIR/rustup"       "$HOME/.rustup"
ln -sfn "$PERSIST_DIR/sccache"      "$HOME/.sccache"
ln -sfn "$PERSIST_DIR/codex"        "$HOME/.codex"
ln -sfn "$PERSIST_DIR/vibe-kanban"  "$HOME/.local/share/vibe-kanban"
ln -sfn "$PERSIST_DIR/gh-config"    "$HOME/.config/gh"
ln -sfn "$PERSIST_DIR/bash_history" "$HOME/.bash_history"
ln -sfn "$PERSIST_DIR/npm"          "$HOME/.npm"
ln -sfn "$PERSIST_DIR/npm-cache"    "$HOME/.cache/npm"
ln -sfn "$PERSIST_DIR/uv-cache"     "$HOME/.cache/uv"
ln -sfn "$PERSIST_DIR/pip-cache"    "$HOME/.cache/pip"
ln -sfn "$PERSIST_DIR/ruff-cache"   "$HOME/.cache/ruff"
sudo mkdir -p /var/tmp/vibe-kanban
sudo chown "$(id -u):$(id -g)" /var/tmp/vibe-kanban
rm -rf /var/tmp/vibe-kanban/worktrees 2>/dev/null || true
ln -sfn "$PERSIST_DIR/vibe-worktrees" /var/tmp/vibe-kanban/worktrees

# Set envvars
export PATH="/usr/local/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
rustup component add rustfmt clippy
rustup set profile default

# Install pre-built binaries (much faster than cargo install)
# -------------------- helper: fetch prebuilt tar/zip into ~/.cargo/bin --------------------
install_bin() {
  local url="$1" name="$2" # name of final binary in PATH
  tmpdir="$(mktemp -d)"; trap 'rm -rf "$tmpdir"' RETURN
  curl -fsSL "$url" -o "$tmpdir/a.tar.gz" || curl -fsSL "$url" -o "$tmpdir/a.tgz" || true
  if [[ -f "$tmpdir/a.tar.gz" || -f "$tmpdir/a.tgz" ]]; then
    tar -xzf "$tmpdir"/a.* -C "$tmpdir" || { echo "tar failed for $name"; return 1; }
    # find the binary anywhere inside the archive
    found="$(find "$tmpdir" -type f -name "$name" -perm -u+x | head -n1 || true)"
    if [[ -n "$found" ]]; then
      install -m 0755 "$found" "$HOME/.cargo/bin/$name"
      echo "installed $name from tarball"
      return 0
    fi
  fi
  # Some projects publish raw single-file binaries
  if [[ ! -f "$HOME/.cargo/bin/$name" ]]; then
    curl -fsSL "$url" -o "$HOME/.cargo/bin/$name" && chmod +x "$HOME/.cargo/bin/$name" && echo "installed $name from raw URL"
  fi
}

SCCACHE_VERSION="0.12.0"
install_bin "https://github.com/mozilla/sccache/releases/download/v${SCCACHE_VERSION}/sccache-v${SCCACHE_VERSION}-x86_64-unknown-linux-musl.tar.gz" sccache || true

DENY_VERSION="0.18.5"
install_bin "https://github.com/EmbarkStudios/cargo-deny/releases/download/${DENY_VERSION}/cargo-deny-${DENY_VERSION}-x86_64-unknown-linux-musl.tar.gz" cargo-deny || true

AUDIT_VERSION="0.21.2"
install_bin "https://github.com/rustsec/rustsec/releases/download/cargo-audit/v${AUDIT_VERSION}/cargo-audit-x86_64-unknown-linux-gnu.tar.gz" cargo-audit || true

NEXTEST_VERSION="0.9.111"
install_bin "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-x86_64-unknown-linux-gnu.tar.gz" cargo-nextest || true

MDBOOK_VERSION="0.4.52"
install_bin "https://github.com/rust-lang/mdBook/releases/download/v${MDBOOK_VERSION}/mdbook-v${MDBOOK_VERSION}-x86_64-unknown-linux-gnu.tar.gz" mdbook || true

export RUSTC_WRAPPER="sccache"
export SCCACHE_DIR="$HOME/.sccache"

# Install Codex CLI
mkdir -p "$HOME/.local/bin" "$HOME/.cache/npm"
npm config set prefix "$HOME/.local"
npm config set cache  "$HOME/.cache/npm"
npm i -g @openai/codex || true

# Verify tools are available
node -v || true
npm -v || true
gh --version || true
rg --version || true
uv --version || true
rustc --version || true
cargo --version || true
for b in sccache cargo-deny cargo-audit cargo-nextest mdbook; do command -v "$b" >/dev/null && "$b" --version || true; done

# Persist exports to .bashrc
grep -q 'RUSTC_WRAPPER' "$HOME/.bashrc" 2>/dev/null || cat >> "$HOME/.bashrc" <<'EOF'
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export RUSTC_WRAPPER="sccache"
export SCCACHE_DIR="$HOME/.sccache"
export RUST_BACKTRACE=1
EOF

echo "✅ Post-create setup completed successfully."