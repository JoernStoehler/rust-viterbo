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


def test_native_stamp_matches_head():
    # The .so must carry a sidecar stamp with the current HEAD commit
    import importlib

    ext = importlib.import_module("viterbo.viterbo_native")
    assert ext.__file__ is not None
    so_path = Path(ext.__file__)
    stamp = so_path.with_name(so_path.name + ".run.json")
    assert stamp.exists(), f"missing native stamp: {stamp}"

    data = json.loads(stamp.read_text())
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"], check=True, capture_output=True, text=True
    ).stdout.strip()
    assert data.get("git_commit") == head, "native .so is stale: stamp git_commit != HEAD"
