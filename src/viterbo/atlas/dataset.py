from __future__ import annotations

from pathlib import Path
from typing import Iterable, Iterator

import polars as pl

from viterbo.provenance import write as write_provenance

from .config import AtlasConfig
from .sources import source_from_spec
from .types import AtlasRow


def build_dataset(cfg: AtlasConfig) -> pl.DataFrame:
    rows = list(_iter_records(cfg))
    if not rows:
        raise ValueError("atlas dataset produced zero rows")
    return pl.DataFrame(rows)


def write_dataset(cfg: AtlasConfig, df: pl.DataFrame) -> Path:
    out_path = cfg.out.dataset
    out_path.parent.mkdir(parents=True, exist_ok=True)
    df.write_parquet(out_path, compression="zstd")
    write_provenance(
        out_path,
        {
            "config_version": cfg.version,
            "seed": cfg.seed,
            "rows": len(df),
        },
        {
            "command": "python -m viterbo.atlas.stage_build --config <file>",
            "exit_code": 0,
        },
    )
    return out_path


def _iter_records(cfg: AtlasConfig) -> Iterator[dict[str, object]]:
    global_row = 0
    for idx, spec in enumerate(cfg.sources):
        seed = cfg.seed + idx * 1_000_003
        source = source_from_spec(spec, seed)
        for row in source.generate():
            yield row.to_record(global_row)
            global_row += 1
