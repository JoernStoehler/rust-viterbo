import json
import os
import subprocess
from pathlib import Path

import polars as pl
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
    dataset_path = Path(cfg["out"]["dataset"])
    sidecar = dataset_path.with_suffix(dataset_path.suffix + ".run.json")
    preview_path = Path(cfg["out"]["preview"])
    assert dataset_path.exists(), "dataset parquet not written"
    assert sidecar.exists(), "provenance sidecar missing"
    assert preview_path.exists(), "preview asset missing"

    df = pl.read_parquet(dataset_path)
    required_cols = {
        "row_id",
        "family",
        "family_name",
        "vertices",
        "halfspaces",
        "volume",
        "systolic_ratio",
    }
    assert required_cols.issubset(set(df.columns))
    hypercube = df.filter(pl.col("family_name").str.contains("hypercube")).to_dicts()
    assert hypercube, "expected hypercube row in special catalog"
    assert hypercube[0]["volume"] == pytest.approx(16.0, rel=1e-6)

    with preview_path.open("r", encoding="utf-8") as handle:
        preview = json.load(handle)
    assert preview["rows"], "preview table should not be empty"
