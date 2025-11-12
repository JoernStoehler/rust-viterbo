# Codex CLI Reference

> **When to read:** pull this up whenever you need to customize Codex CLI behavior, debug sandbox/output quirks, or explain reproducible investigations to another agent. It is intentionally situational, not part of the always-read flow.

## Official Docs & Key Links
- [Configuring Codex](https://developers.openai.com/codex/local-config) – authoritative reference for `~/.codex/config.toml`, profiles, sandbox knobs, and feature flags. The “Configuration options” accordion is the canonical list of supported keys as of November 2025.
- [Codex release notes](https://developers.openai.com/codex/releases) – changelog that often mentions new config fields or behavior changes before they make it into AGENTS.md.

### Configuration File Facts
- The CLI looks for `~/.codex/config.toml` by default, and you can point to alternatives via `CODEX_CONFIG_PATH`. The config merges root keys, named profiles, and CLI flags (flag > profile > root > built-in defaults).[^conf-loc]
- Profiles live under `[profiles.<name>]`; activate one globally with `profile = "<name>"` or ad-hoc via `--profile`. Useful patterns: `big-output` (larger `model_max_output_tokens` + looser sandbox) and `readonly` (force `sandbox_mode="read-only"`).
- Sandbox scopes are controlled by `[sandbox_workspace_write]` allowlists plus `sandbox_mode` (`danger-full-access`, `workspace-write`, `read-only`). If you need to edit outside the repo, add absolute paths to `writable_roots`.
- `[features]` toggles experimental capabilities (`streamable_shell`, `unified_exec`, `web_search_request`, etc.). Legacy `experimental_use_*` keys are deprecated; migrate them before CLI vNext removes the shim.

[^conf-loc]: Source: “Configuration file locations and structure” section in [Configuring Codex](https://developers.openai.com/codex/local-config).

## Investigation Notes (Reproducible Breadcrumbs)

### Shell Output Ceiling
- **Command:** `cat crates/viterbo/src/spec.md` with `max_output_tokens=20000` (2025‑11‑11).
- **Observed:** Output stopped mid-file with `[... output truncated to fit 10240 bytes ...]`.
- **Interpretation:** Even when requesting a larger allowance, the harness enforces a ~10 KB PTY-output cap per `exec_command`. Raising `max_output_tokens` alone cannot bypass this.

### Default vs. Requested `max_output_tokens`
- Running `sed -n '1,200p' crates/viterbo/src/spec.md` without overriding the limit yielded the standard `max_output_tokens=6000` request (visible in `.persist/codex/log/codex-tui.log`). Once the file exceeded that token budget, the CLI truncated the stream and printed `…tokens truncated…`.
- **Mitigation:** Switch to deterministic chunked reads (e.g., `python3 - <<'PY' ...`) or pipe through `head`/`tail` windows sized to stay under the observed ceiling. This preserves full coverage without fighting the hidden cap.

## Practical Tips
- **Large File Reads:** Prefer scripted chunkers (Python loop, `split -l`, `nl | sed`) so every call stays ≤10 KB. Document the chunk size in tickets when reproducibility matters.
- **Config Experiments:** Store alternate configs under `meta/codex-profiles/` (git-ignored) and symlink into `~/.codex/` to avoid polluting personal settings.
- **Logging:** Use `tail -f .persist/codex/log/codex-tui.log` in another terminal when debugging CLI behavior; it logs every tool call with `max_output_tokens`, sandbox mode, and exit codes.
- **Web Restrictions:** When documenting OpenAI-product behavior (like this file), gather citations from official OpenAI domains via Codex web search to satisfy project policy before paraphrasing anything else.

## Open Questions / Future Work
- Can we request a higher PTY buffer via upcoming `streamable_shell` or is the 10 KB cap baked into the harness? Track release notes.
- Document how `web_search_request` interacts with project policies (currently require domain filters for OpenAI-product info).
- Capture more empirical limits (e.g., maximum combined stdout+stderr, long-running command timeout heuristics) as we encounter them.
