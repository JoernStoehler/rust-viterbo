import json
import subprocess
from pathlib import Path
from typing import Any


def test_native_import_and_function():
    # Basic presence + trivial function sanity
    import importlib
    from viterbo import _native

    ext = importlib.import_module("viterbo.viterbo_native")
    assert hasattr(_native, "parallelogram_area")
    # area of (1,0) and (0,1) is +1
    area_fn: Any = getattr(_native, "parallelogram_area")
    assert abs(area_fn((1.0, 0.0), (0.0, 1.0)) - 1.0) < 1e-12
    # extension must load from the repo's src/viterbo/
    assert ext.__file__ is not None
    assert "src/viterbo/" in ext.__file__


def test_volume4_binding_matches_hypercube():
    from viterbo import _native

    hs = []
    for axis in range(4):
        normal = [0.0, 0.0, 0.0, 0.0]
        normal[axis] = 1.0
        hs.append((tuple(normal), 1.0))
        normal[axis] = -1.0
        hs.append((tuple(normal), 1.0))
    vol = getattr(_native, "poly4_volume_from_halfspaces")(hs)
    assert abs(vol - 16.0) < 1e-9


# Intentionally no staleness check:
# We do NOT assert the native .so stamp matches HEAD. Staleness is reliably
# surfaced when a newly added Rust function is called but not present in the
# loaded binary. This avoids forcing rebuilds when unrelated files change.
