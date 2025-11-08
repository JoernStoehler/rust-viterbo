"""Thin Python helpers around the native 4D volume binding.

Why this file exists
- Keep the PyO3 signatures ergonomic for callers that work with Python sequences.
- Provide light validation so that downstream experiments fail fast before
  touching the native module.
"""

from __future__ import annotations

from typing import Callable, Iterable, List, Sequence, Tuple, cast

from viterbo import _native

Halfspace4 = Tuple[Sequence[float], float]
NativeHalfspace = Tuple[Tuple[float, float, float, float], float]


def volume_from_halfspaces(halfspaces: Iterable[Halfspace4]) -> float:
    """Return the 4D volume for an H-representation.

    Args:
        halfspaces: Iterable of ``((n_x, n_y, n_z, n_w), c)`` tuples.

    Returns:
        The hypervolume as a float.
    """

    hs_norm: List[NativeHalfspace] = []
    for n, c in halfspaces:
        if len(n) != 4:
            msg = f"half-space normal must have 4 components, got {len(n)}"
            raise ValueError(msg)
        hs_norm.append(((float(n[0]), float(n[1]), float(n[2]), float(n[3])), float(c)))
    volume_fn = cast(
        Callable[[List[NativeHalfspace]], float],
        getattr(_native, "poly4_volume_from_halfspaces", None),
    )
    if volume_fn is None:
        raise AttributeError("Rust extension missing poly4_volume_from_halfspaces")
    return float(volume_fn(hs_norm))


__all__ = ["volume_from_halfspaces", "Halfspace4"]
