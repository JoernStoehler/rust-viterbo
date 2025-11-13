#!/usr/bin/env bash
# bash-setup.sh — ensure user shell env exports are present
set -euo pipefail

BASHRC="${HOME}/.bashrc"
touch "$BASHRC"

if ! grep -q 'rust-viterbo env exports' "$BASHRC"; then
cat >> "$BASHRC" <<'EOF'
# rust-viterbo env exports
export PATH="$HOME/.elan/bin:$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_HOME="$HOME/.cargo"
export RUSTUP_HOME="$HOME/.rustup"
export RUSTC_WRAPPER="sccache"
export SCCACHE_DIR="$HOME/.sccache"
export RUST_BACKTRACE=1
export CARGO_TARGET_DIR="/var/tmp/vk-target"
EOF
fi
