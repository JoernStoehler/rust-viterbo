#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
ISSUE_DIR="$ROOT/issues"
if [[ ! -d "$ISSUE_DIR" ]]; then
  echo "issues directory not found at $ISSUE_DIR" >&2
  exit 1
fi

ROOT_DIR="$ROOT" ISSUE_DIR="$ISSUE_DIR" python3 <<'PY'
import ast
from pathlib import Path

import os

root = Path(os.environ["ISSUE_DIR"])
files = sorted(p for p in root.glob("*.md"))
for path in files:
    if path.name == "template.md":
        continue
    status = ""
    owners = []
    assignees = []
    tags = []
    created = ""
    updated = ""
    title = ""
    lines = path.read_text().splitlines()
    if not lines:
        continue
    idx = 0
    if lines[idx].strip() == "---":
        idx += 1
        while idx < len(lines):
            line = lines[idx]
            if line.strip() == "---":
                idx += 1
                break
            if ":" in line:
                key, val = line.split(":", 1)
                val = val.split("#", 1)[0].strip()
                key = key.strip()
                if val.startswith("[") and val.endswith("]"):
                    try:
                        parsed = ast.literal_eval(val)
                    except Exception:
                        parsed = []
                    if not isinstance(parsed, list):
                        parsed = [str(parsed)]
                else:
                    parsed = val
                if key == "status":
                    status = str(parsed)
                elif key == "owners":
                    owners = [str(x) for x in parsed]
                elif key == "assignees":
                    assignees = [str(x) for x in parsed]
                elif key == "tags":
                    tags = [str(x) for x in parsed]
                elif key == "created_at":
                    created = str(parsed)
                elif key == "updated_at":
                    updated = str(parsed)
            idx += 1
    for j in range(idx, len(lines)):
        line = lines[j].strip()
        if line.startswith("#"):
            title = line.lstrip("# ")
            break
    owners_str = ",".join(owners)
    assignees_str = ",".join(assignees)
    tags_str = ",".join(tags)
    rel_path = path.relative_to(Path(os.environ["ROOT_DIR"]))
    print(f"{status}\t{owners_str}\t{assignees_str}\t{tags_str}\t{rel_path}\t{title}")
PY
