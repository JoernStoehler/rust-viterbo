#!/usr/bin/env bash
set -euo pipefail

# Run the Vibe Kanban web server in the foreground via npx.
# State directories are persisted via devcontainer postCreate symlinks.
echo "Starting Vibe Kanban (latest) via npx…"
echo "State: $HOME/.local/share/vibe-kanban; Worktrees: /var/tmp/vibe-kanban/worktrees"
exec npx @bloop/vibe-kanban@latest
