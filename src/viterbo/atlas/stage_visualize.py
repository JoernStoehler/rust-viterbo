from __future__ import annotations

import argparse
from pathlib import Path

from .visualize import write_preview


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Create a lightweight preview artifact from an atlas dataset."
    )
    parser.add_argument("--dataset", required=True, help="Path to the dataset parquet.")
    parser.add_argument(
        "--out",
        required=True,
        help="Preview JSON output path (committed under docs/assets/).",
    )
    parser.add_argument(
        "--limit",
        type=int,
        default=32,
        help="Number of rows to keep in the preview table.",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    dataset_path = Path(args.dataset).resolve()
    out_path = Path(args.out).resolve()
    write_preview(dataset_path, out_path, limit=int(args.limit))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
