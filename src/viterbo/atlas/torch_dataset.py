from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

import polars as pl


@dataclass
class AtlasTorchDatasetConfig:
    path: Path
    feature_columns: Sequence[str]
    target_column: str | None = None
    shuffle: bool = False


class AtlasTorchDataset:
    """Tiny helper to feed atlas rows into PyTorch experiments."""

    def __init__(self, cfg: AtlasTorchDatasetConfig) -> None:
        try:
            import torch  # type: ignore
        except ModuleNotFoundError as err:
            raise RuntimeError("torch must be installed to use AtlasTorchDataset") from err

        df = pl.read_parquet(cfg.path)
        if cfg.shuffle:
            df = df.sample(fraction=1.0, with_replacement=False, shuffle=True)
        self._torch = torch
        self._features = _to_tensor(df, cfg.feature_columns, torch)
        self._targets = (
            _to_tensor(df, [cfg.target_column], torch).squeeze(-1) if cfg.target_column else None
        )

    def __len__(self) -> int:
        return self._features.shape[0]

    def __getitem__(self, idx: int):
        if self._targets is None:
            return self._features[idx]
        return self._features[idx], self._targets[idx]


def _to_tensor(df: pl.DataFrame, columns: Sequence[str], torch_module):
    missing = [col for col in columns if col not in df.columns]
    if missing:
        raise ValueError(f"missing columns for tensor conversion: {missing}")
    data = df.select(columns).to_numpy()
    return torch_module.tensor(data, dtype=torch_module.float32)
