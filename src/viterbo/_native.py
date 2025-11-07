"""Thin alias module to expose the Rust extension as viterbo._native.

The actual extension module is named `viterbo_native` (built via maturin).
This shim keeps import paths stable and decouples where the binary lives.
Policy: the native extension is required. We place the built `.so` inside
`src/viterbo/` so it travels with the repo and loads without a build.
"""

from . import viterbo_native as _ext  # type: ignore

# Re-export all public symbols from the extension module.
globals().update({k: v for k, v in vars(_ext).items() if not k.startswith("_")})
