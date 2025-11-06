#!/usr/bin/env bash
# Paper downloader: sync PDFs/arXiv sources listed in docs/src/thesis/bibliography.md
# - Stores everything under data/downloads/(arxiv|web)/...
# - Writes manifest + provenance sidecars for every artifact
# - Falls back to PDFs when arXiv sources are unavailable
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/paper-download.sh --all [options]
  scripts/paper-download.sh --match "pattern" [options]
  scripts/paper-download.sh --arxiv <id> [--title "Title"] [options]
  scripts/paper-download.sh --url <pdf_url> --title "Title" [options]

Options:
  --bib <path>      Override bibliography path (default: docs/src/thesis/bibliography.md)
  --dest <path>     Override output root (default: data/downloads)
  --force           Re-download even if the destination already exists
  --list            Print the parsed bibliography entries and exit
  --help            Show this message

Notes:
  - --match is case-insensitive and must match exactly one bibliography entry.
  - --all iterates over every PDF link in the bibliography.
  - For arXiv links, the script tries to fetch the source tarball first and
    falls back to the PDF if the source is unavailable.
  - Each artifact gets a provenance sidecar via the Python helper
    `viterbo.provenance.write(...)`.
EOF
}

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

need_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "error: required command '$1' not found in PATH" >&2
    exit 1
  fi
}

need_cmd python3   # parsing + slug helpers
need_cmd curl      # downloads
need_cmd gzip      # some arXiv sources arrive as .tar.gz

BIB_PATH="${BIB:-$ROOT_DIR/docs/src/thesis/bibliography.md}"
DEST_ROOT="${DATA_DIR:-$ROOT_DIR/data/downloads}"
MODE=""
PATTERN=""
ARXIV_ID=""
DIRECT_URL=""
CUSTOM_TITLE=""
FORCE=0

require_mode_empty() {
  if [[ -n "$MODE" ]]; then
    echo "error: only one action (--all/--match/--arxiv/--url/--list) may be specified" >&2
    exit 2
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)
      require_mode_empty
      MODE="all"
      shift
      ;;
    --match)
      require_mode_empty
      MODE="match"
      PATTERN="${2:-}"
      if [[ -z "$PATTERN" ]]; then
        echo "error: --match requires a pattern" >&2
        exit 2
      fi
      shift 2
      ;;
    --arxiv)
      require_mode_empty
      MODE="arxiv"
      ARXIV_ID="${2:-}"
      if [[ -z "$ARXIV_ID" ]]; then
        echo "error: --arxiv requires an identifier" >&2
        exit 2
      fi
      shift 2
      ;;
    --url)
      require_mode_empty
      MODE="url"
      DIRECT_URL="${2:-}"
      if [[ -z "$DIRECT_URL" ]]; then
        echo "error: --url requires a value" >&2
        exit 2
      fi
      shift 2
      ;;
    --title)
      CUSTOM_TITLE="${2:-}"
      if [[ -z "$CUSTOM_TITLE" ]]; then
        echo "error: --title requires a value" >&2
        exit 2
      fi
      shift 2
      ;;
    --bib)
      BIB_PATH="${2:-}"
      if [[ -z "$BIB_PATH" ]]; then
        echo "error: --bib requires a path" >&2
        exit 2
      fi
      shift 2
      ;;
    --dest)
      DEST_ROOT="${2:-}"
      if [[ -z "$DEST_ROOT" ]]; then
        echo "error: --dest requires a path" >&2
        exit 2
      fi
      shift 2
      ;;
    --force)
      FORCE=1
      shift
      ;;
    --list)
      require_mode_empty
      MODE="list"
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

if [[ -z "$MODE" ]]; then
  usage
  exit 2
fi

if [[ ! -f "$BIB_PATH" && "$MODE" != "arxiv" && "$MODE" != "url" ]]; then
  echo "error: bibliography not found at $BIB_PATH" >&2
  exit 1
fi

mkdir -p "$DEST_ROOT"

declare -a BIB_ENTRIES=()

load_bibliography() {
  # Extract "title → PDF URL" pairs from the thesis bibliography (stop at Appendix).
  mapfile -t BIB_ENTRIES < <(python3 - "$BIB_PATH" <<'PY'
from pathlib import Path
import sys

if len(sys.argv) < 2:
    raise SystemExit(1)

path = Path(sys.argv[1])
title = None
for raw in path.read_text(encoding="utf-8").splitlines():
    stripped = raw.strip()
    if stripped.startswith("## Appendix"):
        break
    left = raw.lstrip()
    if left.startswith("- ") and not raw.startswith("  -"):
        title = left[2:].strip()
        continue
    if title and left.startswith("- PDF") and ":" in left:
        url = left.split(":", 1)[1].strip()
        if url:
            print(f"{title}\t{url}")
PY
  )
}

slugify() {
  # Deterministic folder names for data/downloads/... entries.
  local input="$1"
  python3 - <<'PY' "$input"
import re
import sys
import unicodedata

text = sys.argv[1]
text = unicodedata.normalize("NFKD", text)
text = text.encode("ascii", "ignore").decode("ascii")
text = re.sub(r"[^a-zA-Z0-9]+", "-", text.lower()).strip("-")
print(text or "paper")
PY
}

relative_path() {
  # Turn absolute paths into repo-relative ones for friendly logging.
  local abs="$1"
  if [[ "$abs" == "$ROOT_DIR"* ]]; then
    local rel="${abs#$ROOT_DIR/}"
    echo "$rel"
  else
    echo "$abs"
  fi
}

declare -a SUMMARY=()
declare -a PDF_ONLY=()

record_summary() {
  SUMMARY+=("$1|$2|$3")
}

record_pdf_only() {
  PDF_ONLY+=("$1")
}

write_provenance() {
  local artifact="$1"
  local params_json="$2"
  [[ -f "$artifact" ]] || return 0
  local tmp
  tmp="$(mktemp)"
  printf '%s' "$params_json" > "$tmp"
  # Use uv to ensure the same Python environment as the rest of the repo.
  uv run python - <<'PY' "$artifact" "$tmp" || echo "warning: failed to write provenance for $artifact" >&2
import json, sys
from pathlib import Path
from viterbo.provenance import write
artifact = Path(sys.argv[1])
params_path = Path(sys.argv[2])
cfg = json.loads(params_path.read_text())
# For downloads, we treat the parsed params as the config.
write(artifact, cfg, {"producer": "scripts/paper-download.sh"})
PY
  rm -f "$tmp"
}

download_file() {
  local url="$1"
  local dest="$2"
  local label="$3"
  local tmp
  tmp="$(mktemp)"
  if curl -fsSL --retry 3 --retry-delay 2 -o "$tmp" "$url"; then
    mv "$tmp" "$dest"
    return 0
  fi
  rm -f "$tmp"
  echo "error: failed to download $label ($url)" >&2
  return 1
}

download_arxiv_source() {
  # arXiv "e-print" endpoint sometimes returns tar.gz, tar, or gzipped TeX.
  # Try the common extraction modes in order and only fall back to PDF if all fail.
  local arxiv_id="$1"
  local dest_dir="$2"
  local archive_path="$dest_dir/source.tar.gz"
  local tmp
  tmp="$(mktemp)"
  if ! curl -fsSL --retry 3 --retry-delay 2 -o "$tmp" "https://arxiv.org/e-print/$arxiv_id"; then
    rm -f "$tmp"
    return 1
  fi
  mv "$tmp" "$archive_path"
  local extract_dir="$dest_dir/src"
  rm -rf "$extract_dir"
  mkdir -p "$extract_dir"
  if tar -xzf "$archive_path" -C "$extract_dir" >/dev/null 2>&1; then
    return 0
  fi
  if tar -xf "$archive_path" -C "$extract_dir" >/dev/null 2>&1; then
    return 0
  fi
  if gzip -dc "$archive_path" > "$extract_dir/main.tex" 2>/dev/null; then
    return 0
  fi
  echo "warning: downloaded arXiv source for $arxiv_id but could not extract" >&2
  rm -f "$archive_path"
  rm -rf "$extract_dir"
  return 1
}

build_manifest_json() {
  local title="$1"
  local url="$2"
  local slug="$3"
  local dest_rel="$4"
  local arxiv_id="$5"
  local text_status="$6"
  local pdf_path="$7"
  local ts
  ts="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  python3 - <<'PY' "$title" "$url" "$slug" "$dest_rel" "$arxiv_id" "$text_status" "$pdf_path" "$ts"
import json
import sys

title, url, slug, dest_rel, arxiv_id, text_status, pdf_path, ts = sys.argv[1:9]
doc = {
    "title": title,
    "url": url,
    "slug": slug,
    "relative_path": dest_rel,
    "downloaded_at": ts,
    "arxiv_id": arxiv_id or None,
    "text_status": text_status,
    "pdf": pdf_path or None,
}
print(json.dumps(doc, indent=2))
PY
}

manifest_text_status() {
  # Small helper so cached entries can show whether they already had sources.
  local manifest="$1"
  python3 - "$manifest" <<'PY' 2>/dev/null
import json
import sys
from pathlib import Path

path = Path(sys.argv[1])
data = json.loads(path.read_text())
print(data.get("text_status", "unknown"))
PY
}

prov_payload() {
  # Keep provenance payloads short and consistent across artifact types.
  local title="$1"
  local kind="$2"
  local source="$3"
  local detail="$4"
  python3 - <<'PY' "$title" "$kind" "$source" "$detail"
import json
import sys

title, kind, source, detail = sys.argv[1:5]
doc = {"title": title, "kind": kind}
if source:
    doc["source"] = source
if detail:
    doc["detail"] = detail
print(json.dumps(doc))
PY
}

download_entry() {
  # Core worker: fetch source + PDF, write manifests, track summary state.
  local title="$1"
  local url="$2"
  local manual_arxiv="$3"
  local slug
  slug="$(slugify "$title")"
  local arxiv_id=""
  if [[ -n "$manual_arxiv" ]]; then
    arxiv_id="$manual_arxiv"
  else
    if [[ "$url" =~ arxiv\.org/(pdf|abs|e\-print)/([^?#]+) ]]; then
      arxiv_id="${BASH_REMATCH[2]}"
      arxiv_id="${arxiv_id%.pdf}"
    fi
  fi
  local dest
  if [[ -n "$arxiv_id" ]]; then
    dest="$DEST_ROOT/arxiv/$arxiv_id"
  else
    dest="$DEST_ROOT/web/$slug"
  fi
  local manifest_path="$dest/manifest.json"
  if [[ -f "$manifest_path" && $FORCE -eq 0 ]]; then
    local cached_status
    cached_status="$(manifest_text_status "$manifest_path" | tr -d '\r')"
    if [[ "$cached_status" == "pdf-only" ]]; then
      record_pdf_only "$title"
    fi
    record_summary "$title" "cached-${cached_status:-unknown}" "$(relative_path "$dest")"
    return 0
  fi
  if [[ $FORCE -eq 1 ]]; then
    rm -rf "$dest"
  fi
  mkdir -p "$dest"

  local text_status="pdf-only"
  local pdf_status="missing"
  local pdf_dest="$dest/paper.pdf"
  local noted_reason=""

  if [[ -n "$arxiv_id" ]]; then
    if download_arxiv_source "$arxiv_id" "$dest"; then
      text_status="arxiv-source"
      write_provenance "$dest/source.tar.gz" "$(prov_payload "$title" "source" "arxiv:$arxiv_id" "$slug")"
    else
      text_status="pdf-only"
      noted_reason="arXiv source unavailable"
      record_pdf_only "$title"
    fi
    if [[ -z "$url" || "$url" == "-" ]]; then
      url="https://arxiv.org/pdf/${arxiv_id}.pdf"
    fi
  else
    record_pdf_only "$title"
  fi

  if [[ -n "$url" ]]; then
    if download_file "$url" "$pdf_dest" "$title PDF"; then
      pdf_status="downloaded"
      write_provenance "$pdf_dest" "$(prov_payload "$title" "pdf" "$url" "$slug")"
    else
      echo "warning: PDF download failed for $title" >&2
    fi
  fi

  local pdf_rel=""
  if [[ -f "$pdf_dest" ]]; then
    pdf_rel="$(relative_path "$pdf_dest")"
  fi
  build_manifest_json "$title" "$url" "$slug" "$(relative_path "$dest")" "$arxiv_id" "$text_status" "$pdf_rel" > "$manifest_path"
  write_provenance "$manifest_path" "$(prov_payload "$title" "manifest" "$url" "$text_status")"

  local status_label="$text_status"
  if [[ "$pdf_status" != "downloaded" ]]; then
    status_label+=" (no-pdf)"
  fi
  if [[ -n "$noted_reason" ]]; then
    status_label+=" - $noted_reason"
  fi
  record_summary "$title" "$status_label" "$(relative_path "$dest")"
  return 0
}

select_entry() {
  local pattern="$1"
  local lower_pattern="${pattern,,}"
  local matches=()
  local entry title
  for entry in "${BIB_ENTRIES[@]}"; do
    title="${entry%%$'\t'*}"
    local lowered="${title,,}"
    if [[ "$lowered" == *"$lower_pattern"* ]]; then
      matches+=("$entry")
    fi
  done
  if [[ ${#matches[@]} -eq 0 ]]; then
    echo "error: no bibliography entry matches '$pattern'" >&2
    exit 1
  fi
  if [[ ${#matches[@]} -gt 1 ]]; then
    echo "error: pattern '$pattern' is ambiguous. Matches:" >&2
    local item
    for item in "${matches[@]}"; do
      echo "  - ${item%%$'\t'*}" >&2
    done
    exit 1
  fi
  echo "${matches[0]}"
}

if [[ "$MODE" == "list" ]]; then
  load_bibliography
  idx=1
  for entry in "${BIB_ENTRIES[@]}"; do
    title="${entry%%$'\t'*}"
    url="${entry#*$'\t'}"
    printf "%2d. %s\n    %s\n" "$idx" "$title" "$url"
    ((idx++))
  done
  exit 0
fi

if [[ "$MODE" == "match" || "$MODE" == "all" ]]; then
  load_bibliography
fi

case "$MODE" in
  all)
    for entry in "${BIB_ENTRIES[@]}"; do
      title="${entry%%$'\t'*}"
      url="${entry#*$'\t'}"
      download_entry "$title" "$url" ""
    done
    ;;
  match)
    selected="$(select_entry "$PATTERN")"
    title="${selected%%$'\t'*}"
    url="${selected#*$'\t'}"
    download_entry "$title" "$url" ""
    ;;
  arxiv)
    title="$CUSTOM_TITLE"
    if [[ -z "$title" ]]; then
      title="arXiv $ARXIV_ID"
    fi
    local_url="https://arxiv.org/pdf/${ARXIV_ID}.pdf"
    download_entry "$title" "$local_url" "$ARXIV_ID"
    ;;
  url)
    if [[ -z "$CUSTOM_TITLE" ]]; then
      echo "error: --url requires --title" >&2
      exit 2
    fi
    download_entry "$CUSTOM_TITLE" "$DIRECT_URL" ""
    ;;
esac

if [[ ${#SUMMARY[@]} -gt 0 ]]; then
  echo
  echo "Summary:"
  for entry in "${SUMMARY[@]}"; do
    IFS='|' read -r title status path <<<"$entry"
    printf " - %s → %s (%s)\n" "$title" "$path" "$status"
  done
fi

if [[ ${#PDF_ONLY[@]} -gt 0 ]]; then
  echo
  echo "PDF-only (no source available):"
  for item in "${PDF_ONLY[@]}"; do
    echo " - $item"
  done
fi

exit 0
