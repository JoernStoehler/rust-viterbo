from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

import polars as pl

from viterbo.provenance import write as write_provenance


@dataclass
class SourceSpec:
    name: str
    rows: int
    max_rows: Optional[int]
    seed: int


def _parse_sources(cfg: Dict[str, Any]) -> List[SourceSpec]:
    base_seed = int(cfg.get("seed", 42))
    entries = cfg.get("sources")
    if entries is None:
        rows = cfg.get("rows")
        if rows is None:
            raise ValueError("config must set either 'sources' or legacy 'rows'")
        entries = [
            {
                "name": "default",
                "rows": rows,
            }
        ]
    if not isinstance(entries, list):
        raise ValueError("'sources' must be a list")
    specs: List[SourceSpec] = []
    for idx, entry in enumerate(entries):
        if not isinstance(entry, dict):
            raise ValueError(f"source #{idx} must be an object, got {type(entry).__name__}")
        name = entry.get("name") or entry.get("generator") or f"source_{idx}"
        rows_val = entry.get("rows")
        rows_source = "rows"
        if rows_val is None:
            rows_val = entry.get("max_rows")
            rows_source = "max_rows"
        if rows_val is None:
            raise ValueError(f"source '{name}' missing 'rows' (or 'max_rows')")
        rows_int = int(rows_val)
        if rows_int < 0:
            raise ValueError(f"source '{name}' has negative {rows_source}")
        if rows_int == 0:
            continue
        max_rows_val = entry.get("max_rows")
        max_rows_int = int(max_rows_val) if max_rows_val is not None else None
        seed_override = entry.get("seed")
        seed_int = int(seed_override) if seed_override is not None else base_seed + idx * 1_000_003
        specs.append(
            SourceSpec(
                name=str(name),
                rows=rows_int,
                max_rows=max_rows_int,
                seed=seed_int,
            )
        )
    if not specs:
        raise ValueError("no sources with positive row counts found in config")
    return specs


def _row_value(seed: int, src_idx: int, row_idx: int) -> int:
    # Deterministic pseudo-random but simple mix.
    return (seed * 131 + src_idx * 479 + row_idx * 53) % 10_007


def build_dataset(cfg: Dict[str, Any]) -> Path:
    sources = _parse_sources(cfg)
    out_path_str = cfg.get("out", {}).get("dataset", "data/atlas/test.parquet")
    out_path = Path(out_path_str)
    out_path.parent.mkdir(parents=True, exist_ok=True)

    records = []
    global_row = 0
    for src_idx, spec in enumerate(sources):
        for row_idx in range(spec.rows):
            value = _row_value(spec.seed, src_idx, row_idx)
            label = "even" if value % 2 == 0 else "odd"
            token = f"{spec.name}:{spec.seed}:{row_idx}"
            records.append(
                {
                    "global_row": global_row,
                    "source": spec.name,
                    "source_row": row_idx,
                    "source_rows": spec.rows,
                    "source_max_rows": spec.max_rows if spec.max_rows is not None else spec.rows,
                    "value": value,
                    "label": label,
                    "replay_token": token,
                }
            )
            global_row += 1
    df = pl.DataFrame(records)
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
