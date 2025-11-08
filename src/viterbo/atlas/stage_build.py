from __future__ import annotations

import argparse
import sys
from pathlib import Path

from .config import AtlasConfig
from .dataset import build_dataset, write_dataset
from .visualize import write_preview


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the atlas dataset.")
    parser.add_argument("--config", required=True, help="Path to the JSON config file.")
    parser.add_argument(
        "--preview-only",
        action="store_true",
        help="Skip dataset generation and refresh only the preview asset.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    config_path = Path(args.config).resolve()
    cfg = AtlasConfig.from_file(config_path)

    if args.preview_only:
        return _run_preview_only(cfg)

    df = build_dataset(cfg)
    dataset_path = write_dataset(cfg, df)
    if cfg.out.preview:
        write_preview(df, cfg.out.preview, limit=cfg.out.preview_limit)
    print(
        f"[atlas] wrote {len(df)} rows to {dataset_path.relative_to(Path.cwd())}",
        file=sys.stderr,
    )
    return 0


def _run_preview_only(cfg: AtlasConfig) -> int:
    if not cfg.out.preview:
        raise ValueError("config does not specify out.preview, preview-only mode invalid")
    dataset_path = cfg.out.dataset
    if not dataset_path.exists():
        raise FileNotFoundError(f"dataset not found: {dataset_path}")
    write_preview(
        dataset_path,
        cfg.out.preview,
        limit=cfg.out.preview_limit,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
