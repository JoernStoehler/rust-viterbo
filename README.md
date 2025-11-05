# Probing Viterbo’s Conjecture — MSc Thesis

[![Thesis](https://img.shields.io/badge/Thesis-Online-2ea44f)](https://JoernStoehler.github.io/rust-viterbo/)

This repository contains the thesis itself (book) and the code that produces its results; it is the authoritative, reproducible source for text, figures, and data artifacts.

## Abstract

We study the systolic ratio on convex polytopes in $\mathbb{R}^4$ to understand where Viterbo’s conjecture fails or nearly holds. The work combines algorithmic computation of Reeb dynamics and actions with data‑driven exploration to map the landscape efficiently and reproducibly.

## Background

For a star‑shaped compact domain $X \subset \mathbb{R}^{2n}$, the systolic ratio is $\mathrm{sys}(X) = A_{\min}(X)^n / \bigl(n!\,\mathrm{vol}(X)\bigr)$, where $A_{\min}(X)$ is the minimal action of a closed Reeb orbit on $\partial X$.  
Viterbo conjectured $\mathrm{sys}(X) \le 1$ for convex $X$. A recent explicit polytope in $\mathbb{R}^4$ violates this bound; our goal is to chart where convex polytopes sit relative to this threshold and why.

References
- [1] J. Chaidez, M. Hutchings, Computing Reeb dynamics on 4d convex polytopes, arXiv:2008.10111.
- [2] P. Haim‑Kislev, Y. Ostrover, A Counterexample to Viterbo’s Conjecture, arXiv:2405.16513.

## Reproduce

Pick an environment; both give the same results.

**Remote** (GitHub Codespaces):
  1) Fork the repo (or use it directly if you have permissions).
  2) Code → Create codespace on main.
  3) Wait for “✅ Post-create setup completed successfully.” in the terminal.

**Local** (VS Code Dev Container):
  1) Clone: `git clone https://github.com/<you>/rust-viterbo && cd rust-viterbo`
  2) Open in VS Code (Dev Containers extension).
  3) “Reopen in Container”.
  4) Wait for “✅ Post-create setup completed successfully.”

After the environment is ready:
1) Run `bash scripts/reproduce.sh` to build the code, tests, data artifacts, and the book.
2) Run `mdbook serve docs -p 4000` and view the book in your browser.

## Onboard

- Quick path: ask your coding agent to read `AGENTS.md` and the documentation, then summarize how to work with the codebase for your task.
- Manual path: start with `AGENTS.md` and `docs/src/meta/overview.md`, later dive into `crates/cli` (orchestration) and `crates/viterbo` (algorithms).

## License

- MIT — see `LICENSE`.
- Acknowledgements: Supervision by K. Cieliebak; project informed by [1] and motivated by [2].
