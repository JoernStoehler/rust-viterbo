# Misc Tools (read when you need helper scripts)

## Paper downloader
- When: you need offline copies of papers cited in `docs/src/thesis/bibliography.md` or you add new bibliography entries.
- Command: `bash scripts/paper-download.sh --list|--match|--all [--force]`.
- Behavior: stores artifacts under `data/downloads/(arxiv|web)/slug/`, fetches arXiv sources when possible (`source.tar.gz`, extracted `src/` tree), downloads PDFs, and writes a `manifest.json` plus provenance sidecars for every file.
- Force-refresh: rerun with `--match "Title" --force` after updating a link (e.g., when you finally obtain the JAMS PDF for Viterbo (2000)).
- Custom URLs: use `--url <pdf> --title "Title"` for items outside the bibliography or paywalled copies you have locally.

Outputs stay gitignored under `data/`; keep only manifests + provenance as breadcrumbs for other agents.
