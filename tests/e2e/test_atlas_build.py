import json
import os
import subprocess
from pathlib import Path

import pytest


@pytest.mark.e2e
def test_build_dataset_tiny(tmp_path: Path):
    config = Path("configs/atlas/test.json")
    assert config.exists(), "missing configs/atlas/test.json"

    # Ensure editable install present for this test run
    subprocess.check_call(["uv", "pip", "install", "-q", "-e", "."])

    cmd = ["python", "-m", "viterbo.atlas.stage_build", "--config", str(config)]
    subprocess.check_call(cmd)

    with open(config, "r", encoding="utf-8") as f:
        cfg = json.load(f)
    out = Path(cfg["out"]["dataset"])
    sidecar = out.with_suffix(out.suffix + ".run.json")
    assert out.exists(), "dataset parquet not written"
    assert sidecar.exists(), "provenance sidecar missing"
