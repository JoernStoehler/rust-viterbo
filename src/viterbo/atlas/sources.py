from __future__ import annotations

import itertools
from typing import Any, Iterator

from viterbo import _native as _native_impl

from .config import SourceConfig
from .types import AtlasRow, PolytopeRecord, build_atlas_row

_native: Any = _native_impl


def source_from_spec(spec: SourceConfig, default_seed: int) -> "AtlasSource":
    factory: dict[str, type[AtlasSource]] = {
        "symmetric_halfspaces": SymmetricHalfspaceSource,
        "mahler_products": MahlerProductSource,
        "regular_products": RegularProductSource,
        "special_catalog": SpecialCatalogSource,
    }
    cls = factory.get(spec.family)
    if cls is None:
        raise ValueError(f"unknown atlas source family '{spec.family}'")
    return cls(spec=spec, default_seed=default_seed)


class AtlasSource:
    spec: SourceConfig
    default_seed: int

    def __init__(self, *, spec: SourceConfig, default_seed: int) -> None:
        self.spec = spec
        self.default_seed = default_seed

    def generate(self) -> Iterator[AtlasRow]:
        raise NotImplementedError

    @property
    def seed(self) -> int:
        return self.spec.seed if self.spec.seed is not None else self.default_seed


class SymmetricHalfspaceSource(AtlasSource):
    def generate(self) -> Iterator[AtlasRow]:
        for idx in range(self.spec.rows):
            sample_seed = self.seed + idx
            poly = _native.rand4_symmetric_halfspace_sample(self.spec.params, int(sample_seed))
            yield build_atlas_row(
                family="symmetric_halfspaces",
                family_name=self.spec.name,
                family_parameters={
                    "params": self.spec.params,
                    "seed": sample_seed,
                },
                replay_token={"seed": sample_seed},
                poly_payload=poly,
            )


class MahlerProductSource(AtlasSource):
    def generate(self) -> Iterator[AtlasRow]:
        base_seed = self.seed
        for idx in range(self.spec.rows):
            poly = _native.rand4_mahler_product_sample(self.spec.params, int(base_seed), int(idx))
            yield build_atlas_row(
                family="mahler_products",
                family_name=self.spec.name,
                family_parameters={
                    "params": self.spec.params,
                    "seed": base_seed,
                    "index": idx,
                },
                replay_token={"seed": base_seed, "index": idx},
                poly_payload=poly,
            )


class RegularProductSource(AtlasSource):
    def generate(self) -> Iterator[AtlasRow]:
        rows_yielded = 0
        pair_index = 0
        while rows_yielded < self.spec.rows:
            maybe_poly = _native.rand4_regular_product_sample(self.spec.params, int(pair_index))
            pair_index += 1
            if maybe_poly is None:
                if rows_yielded == 0:
                    raise ValueError(f"regular product source '{self.spec.name}' produced no rows")
                break
            yield build_atlas_row(
                family="regular_products",
                family_name=self.spec.name,
                family_parameters={
                    "params": self.spec.params,
                    "pair_index": pair_index - 1,
                },
                replay_token={"pair_index": pair_index - 1},
                poly_payload=maybe_poly,
            )
            rows_yielded += 1
        if rows_yielded < self.spec.rows:
            raise ValueError(
                f"regular product source '{self.spec.name}' produced "
                f"{rows_yielded} rows, fewer than requested ({self.spec.rows})"
            )


class SpecialCatalogSource(AtlasSource):
    def generate(self) -> Iterator[AtlasRow]:
        members = self.spec.params.get("members") or []
        if not members:
            raise ValueError(f"special catalog '{self.spec.name}' requires params.members entries")
        for idx in range(self.spec.rows):
            ident = str(members[idx % len(members)])
            record = special_polytope(str(ident))
            payload = {
                "vertices": record.vertices,
                "halfspaces": record.halfspaces,
            }
            yield build_atlas_row(
                family="special_catalog",
                family_name=f"{self.spec.name}:{ident}",
                family_parameters={"member": ident},
                replay_token={"member": ident},
                poly_payload=payload,
            )


def special_polytope(ident: str) -> PolytopeRecord:
    ident = ident.lower()
    if ident == "hypercube":
        return build_hypercube()
    if ident == "cross_polytope":
        return build_cross_polytope()
    if ident == "orthogonal_simplex":
        return build_simplex()
    raise ValueError(f"unknown special catalog member '{ident}'")


def build_hypercube(scale: float = 1.0) -> PolytopeRecord:
    vertices = [[sx, sy, sz, sw] for sx, sy, sz, sw in itertools.product((-scale, scale), repeat=4)]
    halfspaces = []
    for axis in range(4):
        normal = [0.0, 0.0, 0.0, 0.0]
        normal[axis] = 1.0
        halfspaces.append([*normal, scale])
        halfspaces.append([-normal[0], -normal[1], -normal[2], -normal[3], scale])
    return PolytopeRecord(vertices=vertices, halfspaces=halfspaces)


def build_cross_polytope(radius: float = 1.0) -> PolytopeRecord:
    vertices = []
    for axis in range(4):
        for sign in (-radius, radius):
            vertex = [0.0, 0.0, 0.0, 0.0]
            vertex[axis] = sign
            vertices.append(vertex)
    halfspaces = []
    for signs in itertools.product((-1.0, 1.0), repeat=4):
        normal = [signs[0], signs[1], signs[2], signs[3]]
        halfspaces.append([*normal, radius])
    return PolytopeRecord(vertices=vertices, halfspaces=halfspaces)


def build_simplex() -> PolytopeRecord:
    vertices = [
        [0.0, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
    halfspaces = [
        [-1.0, 0.0, 0.0, 0.0, 0.0],
        [0.0, -1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, -1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0, -1.0, 0.0],
        [1.0, 1.0, 1.0, 1.0, 1.0],
    ]
    return PolytopeRecord(vertices=vertices, halfspaces=halfspaces)
