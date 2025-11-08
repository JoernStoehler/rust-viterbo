from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from typing import Any, Mapping, Sequence

from viterbo import _native as _native_impl

_NATIVE: Any = _native_impl


@dataclass
class PolytopeRecord:
    """Simple in-memory representation of a 4D polytope."""

    vertices: list[list[float]] = field(default_factory=list)
    halfspaces: list[list[float]] = field(default_factory=list)

    @property
    def vertex_count(self) -> int:
        return len(self.vertices)

    @property
    def halfspace_count(self) -> int:
        return len(self.halfspaces)


@dataclass
class AtlasRow:
    family: str
    family_name: str
    family_parameters: dict[str, Any]
    replay_token: dict[str, Any]
    polytope: PolytopeRecord
    volume: float
    capacity_ehz: float
    dominant_orbit: str
    systolic_ratio: float

    def to_record(self, row_id: int) -> dict[str, Any]:
        return {
            "row_id": row_id,
            "family": self.family,
            "family_name": self.family_name,
            "family_parameters": json.dumps(self.family_parameters, sort_keys=True),
            "replay_token": json.dumps(self.replay_token, sort_keys=True),
            "vertex_count": self.polytope.vertex_count,
            "halfspace_count": self.polytope.halfspace_count,
            "vertices": self.polytope.vertices,
            "halfspaces": self.polytope.halfspaces,
            "volume": self.volume,
            "capacity_ehz": self.capacity_ehz,
            "dominant_orbit": self.dominant_orbit,
            "systolic_ratio": self.systolic_ratio,
        }


def poly_dict_to_record(payload: Mapping[str, Any]) -> PolytopeRecord:
    vertices = [
        [float(c) for c in seq] for seq in _expect_sequence(payload.get("vertices"), "vertices")
    ]
    halfspaces = [
        [float(c) for c in seq] for seq in _expect_sequence(payload.get("halfspaces"), "halfspaces")
    ]
    return PolytopeRecord(vertices=vertices, halfspaces=halfspaces)


def _halfspaces_for_native(
    poly: PolytopeRecord,
) -> list[tuple[tuple[float, float, float, float], float]]:
    hs_for_native = []
    for h in poly.halfspaces:
        if len(h) != 5:
            raise ValueError("halfspaces must be length-5 lists [n0,n1,n2,n3,c]")
        normal = (float(h[0]), float(h[1]), float(h[2]), float(h[3]))
        hs_for_native.append((normal, float(h[4])))
    return hs_for_native


def compute_volume(poly: PolytopeRecord) -> float:
    """Use the native helper to compute 4D volume from half-spaces."""

    hs_for_native = _halfspaces_for_native(poly)
    try:
        return float(_NATIVE.poly4_volume_from_halfspaces(hs_for_native))
    except Exception:
        return math.nan


def compute_capacity(poly: PolytopeRecord) -> float:
    """Compute c_EHZ using the oriented-edge solver (returns NaN on failure)."""

    hs_for_native = _halfspaces_for_native(poly)
    try:
        result = _NATIVE.poly4_capacity_ehz_from_halfspaces(hs_for_native)
    except Exception:
        return math.nan
    if result is None:
        return math.nan
    if math.isnan(result):
        return math.nan
    return float(result)


def build_atlas_row(
    *,
    family: str,
    family_name: str,
    family_parameters: Mapping[str, Any],
    replay_token: Mapping[str, Any],
    poly_payload: Mapping[str, Any],
    capacity_ehz: float | None = None,
    orbit_label: str | None = None,
) -> AtlasRow:
    record = poly_dict_to_record(poly_payload)
    volume = compute_volume(record)
    if capacity_ehz is not None:
        capacity = float(capacity_ehz)
    else:
        capacity = compute_capacity(record)
    orbit = orbit_label or "unavailable"
    systolic = systolic_ratio(capacity, volume)
    return AtlasRow(
        family=family,
        family_name=family_name,
        family_parameters=dict(family_parameters),
        replay_token=dict(replay_token),
        polytope=record,
        volume=volume,
        capacity_ehz=capacity,
        dominant_orbit=orbit,
        systolic_ratio=systolic,
    )


def systolic_ratio(capacity: float, volume: float) -> float:
    if math.isnan(capacity) or math.isnan(volume) or volume <= 0.0:
        return math.nan
    return (capacity * capacity) / (2.0 * volume)


def _expect_sequence(value: Any, label: str) -> Sequence[Sequence[float]]:
    if not isinstance(value, Sequence):
        raise ValueError(f"{label} must be a list, got {type(value).__name__}")
    return value  # type: ignore[return-value]
