<!-- When to read: Only for rare, owner‑approved history rewrites. Not needed for normal issues. -->

# Git History Cleanup (Situational)

Use only when the owner explicitly requests a repo history rewrite and has scheduled downtime. Escalate first; confirm a backup exists. This page replaces the removed section in AGENTS.md to keep onboarding lean.

Steps (quick guide)
- Pause other worktrees/sessions; get explicit go‑ahead.
- Capture baseline for provenance:
  - `git rev-parse main`
  - `git count-objects -vH`
  - `git lfs ls-files | wc -l`
- Work in a mirror clone:
  - `git clone --mirror /workspaces/rust-viterbo /tmp/history-cleanup.git`
  - `cd /tmp/history-cleanup.git`
- Remove legacy bench artifacts:
  - `git filter-repo --force --invert-paths --path data/bench/release --path data/bench/release/ --path data/bench/tmp --path data/bench/tmp/ --path data/bench/.rustc_info.json --path data/target --path data/target/ --message-callback 'return message + b\"\n[chore] Drop legacy bench artifacts (Issue <uuid>)\n\"'`
- Verify:
  - `git rev-list --objects --all | grep data/bench/release` (should be empty)
  - `git fsck --full`
  - `git lfs fetch --all && git lfs fsck`
- Publish to staging:
  - Push rewritten result to a staging branch in `/workspaces/rust-viterbo` (e.g., `main-clean`).
  - Owner force‑pushes to GitHub; local worktrees rehydrate LFS:
    - `git lfs pull --include "data/**" --exclude ""`
  - Run quick loops:
    - `bash scripts/python-lint-type-test.sh`
    - `bash scripts/rust-test.sh`
- Record actions: commands run and key hashes in the issue; no extra files are required for the rewrite itself.

Notes
- Do not attempt without owner approval and a quiet window; history rewrites disrupt collaborators’ clones and LFS pointers.
- Prefer targeted filters over broad path patterns; test in a mirror clone first.
