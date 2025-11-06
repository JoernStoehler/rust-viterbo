from __future__ import annotations

import json
import os
import shlex
import subprocess
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, Mapping, MutableMapping, Optional


def _git_rev() -> Optional[str]:
    try:
        out = subprocess.check_output(["git", "rev-parse", "--short=12", "HEAD"], text=True).strip()
        return out or None
    except Exception:
        return None


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def _sidecar_path(output_path: Path) -> Path:
    return output_path.with_suffix(output_path.suffix + ".run.json")


def write(
    output_path: os.PathLike[str] | str,
    config: Mapping[str, Any],
    extras: Optional[Mapping[str, Any]] = None,
) -> Path:
    """
    Write a small JSON sidecar next to an artifact.
    Always writes `<artifact>.<ext>.run.json` and embeds the (possibly mutated) config.
    """
    out = Path(output_path)
    sidecar = _sidecar_path(out)
    sidecar.parent.mkdir(parents=True, exist_ok=True)

    payload: Dict[str, Any] = {
        "config": dict(config),
        "git_commit": _git_rev(),
        "timestamp": _now_iso(),
    }
    if extras:
        payload.update(extras)

    tmp = sidecar.with_suffix(sidecar.suffix + ".tmp")
    with tmp.open("w", encoding="utf-8") as f:
        json.dump(payload, f, indent=2, sort_keys=True)
        f.write("\n")
    tmp.replace(sidecar)
    return sidecar
