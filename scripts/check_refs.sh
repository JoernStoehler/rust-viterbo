#!/usr/bin/env bash
set -euo pipefail

# Collect TH anchors referenced from code
grep -Rho 'TH:\s*[A-Za-z0-9._-]\+' crates | awk '{print $2}' | sort -u > /tmp/th_from_code.txt || true
# Collect TH anchors present in thesis headings {#anchor}
grep -Rho '{#[A-Za-z0-9._-]\+}' docs/src/thesis | tr -d '{}' | cut -d# -f2 | sort -u > /tmp/th_in_thesis.txt || true

missing=$(comm -23 /tmp/th_from_code.txt /tmp/th_in_thesis.txt || true)
if [[ -n "$missing" ]]; then
  echo "Missing TH anchors in thesis:"
  echo "$missing"
  exit 1
fi

# Verify VK ids exist via vk.sh if supported; otherwise skip with a notice
if grep -q "exists)" scripts/vk.sh 2>/dev/null; then
  grep -Rho 'VK:\s*[0-9a-f-]\{36\}' crates docs/src | awk '{print $2}' | sort -u | while read -r id; do
    bash scripts/vk.sh exists "$id" || { echo "Unknown VK: $id"; exit 1; }
  done
else
  echo "Skipping VK id existence check (vk.sh has no 'exists' command)"
fi

echo "References OK"
