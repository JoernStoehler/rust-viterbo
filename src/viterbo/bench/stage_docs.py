"""Pipeline stage: convert Criterion benchmarks into docs assets.

Usage:
  uv run python -m viterbo.bench.stage_docs --config configs/bench/docs_local.json
Optional overrides: --bench-root, --assets-root, --keep.
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import platform
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CONFIG = REPO_ROOT / "configs" / "bench" / "docs_local.json"


@dataclass
class StageConfig:
    bench_root: Path
    assets_root: Path
    keep: int = 5

    @classmethod
    def from_json(cls, path: Path) -> StageConfig:
        data = json.loads(path.read_text())
        bench_root = resolve_path(data.get("bench_root", "data/bench/criterion"))
        assets_root = resolve_path(data.get("assets_root", "docs/assets/bench"))
        keep = int(data.get("keep", 5))
        return cls(bench_root=bench_root, assets_root=assets_root, keep=keep)


def resolve_path(raw: str | Path) -> Path:
    path = Path(raw)
    if not path.is_absolute():
        path = (REPO_ROOT / path).resolve()
    return path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Render Criterion summaries for docs")
    parser.add_argument(
        "--config",
        type=Path,
        default=DEFAULT_CONFIG,
        help="JSON config with bench_root/assets_root/keep (default: configs/bench/docs_local.json)",
    )
    parser.add_argument("--bench-root", type=Path, help="Override Criterion root directory")
    parser.add_argument("--assets-root", type=Path, help="Override docs assets directory")
    parser.add_argument("--keep", type=int, help="Override retention window (files per group)")
    return parser.parse_args()


def gather_context() -> dict[str, str]:
    timestamp = dt.datetime.now(dt.timezone.utc)
    git_commit = run_command(["git", "rev-parse", "HEAD"]) or "unknown"
    rustc = run_command(["rustc", "--version"])
    return {
        "timestamp": timestamp.isoformat(),
        "timestamp_label": timestamp.strftime("%Y-%m-%d %H:%M:%SZ"),
        "git_commit": git_commit,
        "git_short": git_commit[:7] if git_commit else "",
        "hostname": platform.node(),
        "cpu": platform.processor() or platform.machine(),
        "os": f"{platform.system()} {platform.release()}".strip(),
        "rustc": rustc or "",
    }


def run_command(cmd: List[str]) -> str:
    try:
        return subprocess.run(cmd, check=True, capture_output=True, text=True).stdout.strip()
    except (subprocess.CalledProcessError, FileNotFoundError):
        return ""


def collect_group_rows(group_dir: Path, context: dict[str, str]) -> List[dict[str, object]]:
    rows: List[dict[str, object]] = []
    for estimates_path in sorted(group_dir.glob("**/new/estimates.json")):
        rel = estimates_path.relative_to(group_dir)
        parts = rel.parts
        if len(parts) >= 4:
            bench_name, parameter = parts[0], parts[1]
        elif len(parts) >= 3:
            # Some benches (e.g., volume4) only encode the parameter level.
            bench_name, parameter = group_dir.name, parts[0]
        else:
            continue
        estimates = json.loads(estimates_path.read_text())
        mean_ns = estimates.get("mean", {}).get("point_estimate")
        std_ns = estimates.get("std_dev", {}).get("point_estimate")
        median_ns = estimates.get("median", {}).get("point_estimate")

        sample_file = estimates_path.parent / "sample.json"
        min_ns, sample_count = None, 0
        if sample_file.exists():
            sample = json.loads(sample_file.read_text())
            iters = sample.get("iters", [])
            totals = sample.get("total_times") or sample.get("times", [])
            sample_count = min(len(iters), len(totals))
            per_sample = [totals[i] / iters[i] for i in range(sample_count) if iters[i]]
            if per_sample:
                min_ns = min(per_sample)

        rows.append(
            {
                "timestamp": context["timestamp"],
                "git_commit": context["git_commit"],
                "group": group_dir.name,
                "bench": bench_name,
                "parameter": parameter,
                "samples": sample_count,
                "min_ns": min_ns,
                "mean_ns": mean_ns,
                "stddev_ns": std_ns,
                "median_ns": median_ns,
                "hostname": context["hostname"],
                "cpu": context["cpu"],
                "os": context["os"],
                "rustc": context["rustc"],
            }
        )
    rows.sort(key=row_sort_key)
    return rows


def row_sort_key(row: dict[str, object]):
    bench = str(row.get("bench", ""))
    parameter = row.get("parameter")
    if isinstance(parameter, (int, float)):
        parameter_value = float(parameter)
    else:
        try:
            parameter_value = float(str(parameter))
        except (TypeError, ValueError):
            parameter_value = str(parameter)
    return (bench, parameter_value)


def write_csv(rows: Iterable[dict[str, object]], destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    fieldnames = [
        "timestamp",
        "git_commit",
        "group",
        "bench",
        "parameter",
        "samples",
        "min_ns",
        "mean_ns",
        "stddev_ns",
        "median_ns",
        "hostname",
        "cpu",
        "os",
        "rustc",
    ]
    with destination.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def write_markdown(
    rows: List[dict[str, object]], destination: Path, context: dict[str, str]
) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        "| bench | parameter | samples | min (ns) | mean (ns) | stddev (ns) |",
        "| --- | --- | ---: | ---: | ---: | ---: |",
    ]
    for row in rows:
        lines.append(
            "| {bench} | {param} | {samples} | {min_val:.3f} | {mean:.3f} | {std:.3f} |".format(
                bench=row["bench"],
                param=row["parameter"],
                samples=row["samples"],
                min_val=(row["min_ns"] or 0.0),
                mean=(row["mean_ns"] or 0.0),
                std=(row["stddev_ns"] or 0.0),
            )
        )
    lines.append("")
    lines.append(
        f"_Updated {context['timestamp_label']} · commit {context['git_short']} · host {context['hostname']} · rustc {context['rustc']}_"
    )
    destination.write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_provenance(
    csv_path: Path, context: dict[str, str], rows: List[dict[str, object]]
) -> None:
    payload = {
        "git_commit": context["git_commit"],
        "timestamp": context["timestamp"],
        "row_count": len(rows),
        "group": rows[0]["group"] if rows else None,
        "hostname": context["hostname"],
        "cpu": context["cpu"],
        "os": context["os"],
        "rustc": context["rustc"],
    }
    csv_path.with_suffix(csv_path.suffix + ".run.json").write_text(
        json.dumps(payload, indent=2),
        encoding="utf-8",
    )


def prune_history(group: str, assets_root: Path, keep: int, extension: str) -> None:
    keep = max(keep, 1)
    pattern = f"*_{group}{extension}"
    files = sorted(assets_root.glob(pattern))
    for old in files[:-keep]:
        old.unlink(missing_ok=True)
        if extension == ".csv":
            sidecar = old.with_suffix(old.suffix + ".run.json")
            sidecar.unlink(missing_ok=True)


def update_symlink(link_path: Path, target_name: str) -> None:
    if link_path.exists() or link_path.is_symlink():
        link_path.unlink()
    link_path.symlink_to(target_name)


def run_stage(cfg: StageConfig, context: dict[str, str]) -> None:
    bench_root = cfg.bench_root
    assets_root = cfg.assets_root
    if not bench_root.exists():
        raise FileNotFoundError(f"bench root {bench_root} does not exist")

    timestamp_tag = dt.datetime.fromisoformat(context["timestamp"]).strftime("%Y%m%dT%H%M%SZ")
    generated_groups: List[tuple[str, Path]] = []
    # Mirror location under mdBook src so {{#include}} resolves without preprocessor errors.
    # We only mirror the rolling 'current_<group>.md' file (tiny) to keep src tidy.
    assets_src_md_root = REPO_ROOT / "docs" / "src" / "assets" / "bench"
    assets_src_md_root.mkdir(parents=True, exist_ok=True)

    for group_dir in sorted(p for p in bench_root.iterdir() if p.is_dir()):
        rows = collect_group_rows(group_dir, context)
        if not rows:
            continue
        csv_path = assets_root / f"{timestamp_tag}_{group_dir.name}.csv"
        md_path = assets_root / f"{timestamp_tag}_{group_dir.name}.md"
        write_csv(rows, csv_path)
        write_markdown(rows, md_path, context)
        write_provenance(csv_path, context, rows)
        update_symlink(assets_root / f"current_{group_dir.name}.csv", csv_path.name)
        update_symlink(assets_root / f"current_{group_dir.name}.md", md_path.name)
        # Write/overwrite the mirrored mdBook-src copy with the current snapshot content.
        mirror_md = assets_src_md_root / f"current_{group_dir.name}.md"
        mirror_md.write_text(md_path.read_text(encoding="utf-8"), encoding="utf-8")
        prune_history(group_dir.name, assets_root, cfg.keep, ".csv")
        prune_history(group_dir.name, assets_root, cfg.keep, ".md")
        try:
            relative = csv_path.relative_to(REPO_ROOT)
        except ValueError:
            relative = csv_path
        print(f"[bench.stage_docs] wrote {relative} ({len(rows)} rows)")
        generated_groups.append((group_dir.name, csv_path))

    if generated_groups:
        update_symlink(assets_root / "current.csv", generated_groups[0][1].name)
    else:
        raise RuntimeError(f"no benchmark rows found under {bench_root}")


def main() -> int:
    args = parse_args()
    config_path = resolve_path(args.config)
    if not config_path.exists():
        print(f"error: config {config_path} does not exist", file=sys.stderr)
        return 2
    cfg = StageConfig.from_json(config_path)
    if args.bench_root is not None:
        cfg.bench_root = resolve_path(args.bench_root)
    if args.assets_root is not None:
        cfg.assets_root = resolve_path(args.assets_root)
    if args.keep is not None:
        cfg.keep = args.keep

    context = gather_context()
    try:
        run_stage(cfg, context)
    except Exception as exc:  # noqa: BLE001
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
