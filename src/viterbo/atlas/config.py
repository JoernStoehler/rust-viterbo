from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, MutableMapping, Sequence


@dataclass(frozen=True)
class OutputConfig:
    dataset: Path
    preview: Path | None = None
    preview_limit: int = 32


@dataclass(frozen=True)
class SourceConfig:
    name: str
    family: str
    rows: int
    params: dict[str, Any]
    seed: int | None = None


@dataclass(frozen=True)
class AtlasConfig:
    version: int
    seed: int
    sources: list[SourceConfig]
    out: OutputConfig

    @classmethod
    def from_mapping(
        cls,
        data: Mapping[str, Any],
        *,
        base_dir: Path,
    ) -> "AtlasConfig":
        version = int(data.get("version", 1))
        seed = int(data.get("seed", 0))
        out_cfg = cls._parse_out(data.get("out", {}), base_dir)
        sources = cls._parse_sources(data.get("sources", []))
        if not sources:
            raise ValueError("config must provide at least one source")
        return cls(version=version, seed=seed, sources=sources, out=out_cfg)

    @classmethod
    def from_file(cls, path: Path) -> "AtlasConfig":
        import json

        with path.open("r", encoding="utf-8") as handle:
            payload = json.load(handle)
        return cls.from_mapping(payload, base_dir=path.parent)

    @staticmethod
    def _parse_out(payload: Mapping[str, Any], base_dir: Path) -> OutputConfig:
        dataset_raw = payload.get("dataset")
        if not dataset_raw:
            raise ValueError("out.dataset must be set")
        dataset = _resolve_path(dataset_raw, base_dir)
        preview_raw = payload.get("preview")
        preview = _resolve_path(preview_raw, base_dir) if preview_raw else None
        preview_limit = int(payload.get("preview_limit", 32))
        return OutputConfig(dataset=dataset, preview=preview, preview_limit=preview_limit)

    @staticmethod
    def _parse_sources(payload: Any) -> list[SourceConfig]:
        if not isinstance(payload, Sequence):
            raise ValueError("'sources' must be a list")
        specs: list[SourceConfig] = []
        for entry in payload:
            if not isinstance(entry, Mapping):
                raise ValueError("each source entry must be an object")
            entry_mut: MutableMapping[str, Any] = dict(entry)
            name = str(entry_mut.get("name") or entry_mut.get("family"))
            family = str(entry_mut.get("family") or entry_mut.get("name"))
            rows = AtlasConfig._infer_row_count(entry_mut)
            params = dict(entry_mut.get("params") or {})
            seed = entry_mut.get("seed")
            specs.append(
                SourceConfig(
                    name=name,
                    family=family,
                    rows=rows,
                    params=params,
                    seed=int(seed) if seed is not None else None,
                )
            )
        return specs

    @staticmethod
    def _infer_row_count(entry: Mapping[str, Any]) -> int:
        rows = entry.get("rows")
        if rows is not None:
            rows_int = int(rows)
            if rows_int <= 0:
                raise ValueError(f"source '{entry.get('name')}' must have positive rows")
            return rows_int
        params = entry.get("params") or {}
        members = params.get("members")
        if isinstance(members, Sequence) and members:
            return len(members)
        raise ValueError(f"source '{entry.get('name')}' missing 'rows' and no inferable count")


def _resolve_path(candidate: str, base_dir: Path) -> Path:
    path = Path(candidate)
    if path.is_absolute():
        return path
    # Prefer project root (cwd) over config dir so configs under configs/ can
    # reference paths like data/... without adding ../ prefixes.
    return (Path.cwd() / path).resolve()
