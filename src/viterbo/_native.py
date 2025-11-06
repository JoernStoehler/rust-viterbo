"""Thin alias module to expose the Rust extension as viterbo._native.

The actual extension module is named `viterbo_native` (built via maturin).
This shim keeps import paths stable and decouples packaging details.
"""

try:
    from viterbo_native import *  # noqa: F401,F403
except Exception as _e:  # pragma: no cover
    # Optional dependency: pipelines can run without the native module.
    raise
