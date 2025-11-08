from __future__ import annotations

import json
from pathlib import Path
from typing import Iterable, Sequence

import polars as pl

DEFAULT_PREVIEW_COLUMNS = [
    "row_id",
    "family",
    "family_name",
    "vertex_count",
    "halfspace_count",
    "volume",
    "capacity_ehz",
    "systolic_ratio",
]


def write_preview(
    dataset: pl.DataFrame | Path,
    out_path: Path,
    *,
    limit: int = 32,
    columns: Sequence[str] | None = None,
) -> Path:
    df = dataset if isinstance(dataset, pl.DataFrame) else pl.read_parquet(dataset)
    cols = list(columns) if columns is not None else DEFAULT_PREVIEW_COLUMNS
    missing = [col for col in cols if col not in df.columns]
    if missing:
        raise ValueError(f"preview columns missing from dataframe: {missing}")
    preview_df = df.select(cols).head(limit)
    float_exprs = [
        pl.col(name).fill_nan(None)
        for name, dtype in zip(preview_df.columns, preview_df.dtypes)
        if dtype in (pl.Float32, pl.Float64)
    ]
    if float_exprs:
        preview_df = preview_df.with_columns(float_exprs)
    preview_payload = {
        "columns": cols,
        "rows": preview_df.to_dicts(),
        "row_limit": limit,
    }
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(preview_payload, indent=2), encoding="utf-8")
    return out_path
