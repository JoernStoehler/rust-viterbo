from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any, Dict

import polars as pl

from viterbo.provenance import write as write_provenance


def build_dataset(cfg: Dict[str, Any]) -> Path:
    rows = int(cfg.get("rows", 10))
    seed = int(cfg.get("seed", 42))
    out_path_str = cfg.get("out", {}).get("dataset", "data/atlas/test.parquet")
    out_path = Path(out_path_str)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # Dummy dataset: simple numeric pattern; deterministic given rows/seed
    ids = list(range(rows))
    values = [((i * 2 + seed) % 97) for i in ids]
    labels = [("even" if ((i + seed) % 2 == 0) else "odd") for i in ids]
    df = pl.DataFrame({"id": ids, "value": values, "label": labels})
    df.write_parquet(out_path)

    # Provenance sidecar with embedded config and minimal runtime info
    write_provenance(
        out_path,
        cfg,
        {
            "command": "python -m viterbo.atlas.stage_build --config <file>",
            "exit_code": 0,
        },
    )
    return out_path


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Build the tiny atlas dataset.")
    parser.add_argument("--config", required=True, help="Path to JSON config file.")
    args = parser.parse_args(argv)

    with open(args.config, "r", encoding="utf-8") as f:
        cfg = json.load(f)
    try:
        build_dataset(cfg)
    except Exception as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
