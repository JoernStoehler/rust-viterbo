//! Random and enumerative 4D polytope generators.
//!
//! Purpose
//! - Provide reproducible polytope streams for the atlas dataset and other experiments.
//! - Encode the conventions from TH: docs/src/thesis/random-polytopes.md.
//!
//! Why this design
//! - Every sample carries the params snapshot plus a replay token (seed or index tuple).
//! - `PolytopeGenerator4` exposes both streaming (`generate_next`) and replay (`regenerate`)
//!   entry points so orchestrators can take either path without duplicating logic.
//! - Implementations stay small and explicit (paired halfspaces, regular polygon products).
//!
//! References
//! - TH: docs/src/thesis/random-polytopes.md
//! - Ticket: 0f48-random-polytopes

use crate::geom2::{
    rand::{
        draw_polygon_radial, polar as polar_poly2, recenter_rescale, Bounds2, RadialCfg,
        ReplayToken as Poly2ReplayToken, VertexCount,
    },
    Poly2,
};
use crate::geom4::{Hs4, Poly4};
use nalgebra::{Matrix4, Vector2, Vector4};
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};
use std::fmt;

/// Error type shared by all generators.
#[derive(Debug)]
pub enum GeneratorError {
    InvalidParams { reason: String },
    DegenerateSample { reason: String },
}

impl GeneratorError {
    fn invalid(reason: impl Into<String>) -> Self {
        Self::InvalidParams {
            reason: reason.into(),
        }
    }

    fn degenerate(reason: impl Into<String>) -> Self {
        Self::DegenerateSample {
            reason: reason.into(),
        }
    }
}

impl fmt::Display for GeneratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidParams { reason } => write!(f, "invalid generator params: {reason}"),
            Self::DegenerateSample { reason } => write!(f, "degenerate sample: {reason}"),
        }
    }
}

impl std::error::Error for GeneratorError {}

/// A single polytope row plus replay metadata.
#[derive(Clone, Debug)]
pub struct PolytopeSample4<P, R> {
    pub polytope: Poly4,
    pub params: P,
    pub replay: R,
}

/// Common trait for reproducible polytope sources.
pub trait PolytopeGenerator4 {
    type Params: Clone;
    type Replay: Clone;

    fn params(&self) -> &Self::Params;

    fn generate_next(
        &mut self,
    ) -> Result<Option<PolytopeSample4<Self::Params, Self::Replay>>, GeneratorError>;

    fn regenerate(&self, replay: &Self::Replay) -> Result<Poly4, GeneratorError>;
}

/// Parameters for centrally symmetric random halfspaces.
#[derive(Clone, Debug)]
pub struct SymmetricHalfspaceParams {
    pub directions: usize,
    pub radius_min: f64,
    pub radius_max: f64,
    pub anisotropy: Option<Matrix4<f64>>,
}

impl SymmetricHalfspaceParams {
    fn validate(&self) -> Result<(), GeneratorError> {
        if self.directions == 0 {
            return Err(GeneratorError::invalid("need at least one direction"));
        }
        if !(self.radius_min.is_finite() && self.radius_max.is_finite()) {
            return Err(GeneratorError::invalid("radius bounds must be finite"));
        }
        if self.radius_min <= 0.0 {
            return Err(GeneratorError::invalid("radius_min must be > 0"));
        }
        if self.radius_min > self.radius_max {
            return Err(GeneratorError::invalid("radius_min <= radius_max required"));
        }
        Ok(())
    }
}

/// Replay token storing the seed that regenerates the same halfspace sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedReplay {
    pub seed: u64,
}

/// Generator implementing the “centrally symmetric random halfspaces” family.
pub struct SymmetricHalfspaceGenerator {
    params: SymmetricHalfspaceParams,
    master_rng: StdRng,
}

impl SymmetricHalfspaceGenerator {
    pub fn new(params: SymmetricHalfspaceParams, seed: u64) -> Result<Self, GeneratorError> {
        params.validate()?;
        Ok(Self {
            params,
            master_rng: StdRng::seed_from_u64(seed),
        })
    }

    pub fn generate_single(
        params: &SymmetricHalfspaceParams,
        seed: u64,
    ) -> Result<Poly4, GeneratorError> {
        params.validate()?;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut hs = Vec::with_capacity(params.directions * 2);
        for _ in 0..params.directions {
            let dir = sample_unit_vector(&mut rng);
            let dir = match &params.anisotropy {
                Some(mat) => {
                    let transformed = mat * dir;
                    normalize_vector(transformed).ok_or_else(|| {
                        GeneratorError::degenerate("anisotropy map produced a zero direction")
                    })?
                }
                None => dir,
            };
            let radius = sample_radius(&mut rng, params.radius_min, params.radius_max);
            hs.push(Hs4::new(dir, radius));
            hs.push(Hs4::new(-dir, radius));
        }
        Ok(Poly4::from_h(hs))
    }
}

impl PolytopeGenerator4 for SymmetricHalfspaceGenerator {
    type Params = SymmetricHalfspaceParams;
    type Replay = SeedReplay;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn generate_next(
        &mut self,
    ) -> Result<Option<PolytopeSample4<Self::Params, Self::Replay>>, GeneratorError> {
        let sample_seed = self.master_rng.next_u64();
        let poly = Self::generate_single(&self.params, sample_seed)?;
        Ok(Some(PolytopeSample4 {
            polytope: poly,
            params: self.params.clone(),
            replay: SeedReplay { seed: sample_seed },
        }))
    }

    fn regenerate(&self, replay: &Self::Replay) -> Result<Poly4, GeneratorError> {
        Self::generate_single(&self.params, replay.seed)
    }
}

/// Parameters for the Mahler product generator (K × K° in R⁴).
#[derive(Clone, Debug)]
pub struct MahlerProductParams {
    pub radial_cfg: RadialCfg,
    pub bounds: Bounds2,
    pub max_attempts: u32,
}

impl MahlerProductParams {
    fn validate(&self) -> Result<(), GeneratorError> {
        if self.radial_cfg.base_radius <= 0.0 || !self.radial_cfg.base_radius.is_finite() {
            return Err(GeneratorError::invalid(
                "radial_cfg.base_radius must be finite and positive",
            ));
        }
        if self.bounds.r_in_min <= 0.0 {
            return Err(GeneratorError::invalid("bounds.r_in_min must be > 0"));
        }
        if self.bounds.r_out_max > 0.0 && self.bounds.r_out_max <= self.bounds.r_in_min {
            return Err(GeneratorError::invalid(
                "bounds.r_out_max must exceed bounds.r_in_min when both are set",
            ));
        }
        if self.max_attempts == 0 {
            return Err(GeneratorError::invalid("max_attempts must be > 0"));
        }
        Ok(())
    }
}

impl Default for MahlerProductParams {
    fn default() -> Self {
        Self {
            radial_cfg: RadialCfg {
                vertex_count: VertexCount::Fixed(12),
                angle_jitter_frac: 0.25,
                radial_jitter: 0.2,
                base_radius: 1.0,
                random_phase: true,
            },
            bounds: Bounds2 {
                r_in_min: 0.1,
                r_out_max: 2.0,
            },
            max_attempts: 16,
        }
    }
}

/// Replay token for Mahler samples (wraps the 2D polygon token).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MahlerReplay {
    pub polygon: Poly2ReplayToken,
}

/// Generator producing Mahler products `K × K°`.
pub struct MahlerProductGenerator {
    params: MahlerProductParams,
    seed: u64,
    next_index: u64,
}

impl MahlerProductGenerator {
    pub fn new(params: MahlerProductParams, seed: u64) -> Result<Self, GeneratorError> {
        params.validate()?;
        Ok(Self {
            params,
            seed,
            next_index: 0,
        })
    }

    fn sample_with_token(
        params: &MahlerProductParams,
        token: Poly2ReplayToken,
    ) -> Result<Poly4, GeneratorError> {
        let poly_raw = draw_polygon_radial(params.radial_cfg, token).ok_or_else(|| {
            GeneratorError::degenerate("radial sampler returned a degenerate polygon")
        })?;
        let (poly_centered, _, _) =
            recenter_rescale(&poly_raw, params.bounds).ok_or_else(|| {
                GeneratorError::degenerate("recenter/rescale failed for requested bounds")
            })?;
        let polar = polar_poly2(&poly_centered).ok_or_else(|| {
            GeneratorError::degenerate("polar construction failed (origin not interior?)")
        })?;
        Ok(cartesian_product(&poly_centered, &polar))
    }
}

impl PolytopeGenerator4 for MahlerProductGenerator {
    type Params = MahlerProductParams;
    type Replay = MahlerReplay;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn generate_next(
        &mut self,
    ) -> Result<Option<PolytopeSample4<Self::Params, Self::Replay>>, GeneratorError> {
        let attempts = self.params.max_attempts.max(1) as usize;
        for _ in 0..attempts {
            let token = Poly2ReplayToken {
                seed: self.seed,
                index: self.next_index,
            };
            self.next_index = self.next_index.wrapping_add(1);
            match Self::sample_with_token(&self.params, token) {
                Ok(poly) => {
                    return Ok(Some(PolytopeSample4 {
                        polytope: poly,
                        params: self.params.clone(),
                        replay: MahlerReplay { polygon: token },
                    }))
                }
                Err(GeneratorError::DegenerateSample { .. }) => continue,
                Err(err) => return Err(err),
            }
        }
        Err(GeneratorError::degenerate(
            "Mahler sampler exceeded max_attempts without a valid sample",
        ))
    }

    fn regenerate(&self, replay: &Self::Replay) -> Result<Poly4, GeneratorError> {
        Self::sample_with_token(&self.params, replay.polygon)
    }
}

fn sample_unit_vector(rng: &mut StdRng) -> Vector4<f64> {
    loop {
        let v = Vector4::new(
            sample_component(rng),
            sample_component(rng),
            sample_component(rng),
            sample_component(rng),
        );
        if let Some(normalized) = normalize_vector(v) {
            return normalized;
        }
    }
}

fn sample_component(rng: &mut StdRng) -> f64 {
    // Uniform in [-1, 1].
    let raw = rng.next_u64();
    // Convert to f64 in [0,1).
    let unit = (raw >> 11) as f64 / ((1u64 << 53) as f64);
    unit * 2.0 - 1.0
}

fn normalize_vector(v: Vector4<f64>) -> Option<Vector4<f64>> {
    let norm = v.norm();
    if norm < 1e-12 {
        None
    } else {
        Some(v / norm)
    }
}

fn sample_radius(rng: &mut StdRng, min: f64, max: f64) -> f64 {
    if (max - min).abs() < f64::EPSILON {
        return min;
    }
    let unit = rng.next_u64() as f64 / (u64::MAX as f64);
    min + (max - min) * unit
}

fn cartesian_product(lhs: &Poly2, rhs: &Poly2) -> Poly4 {
    let mut hs = Vec::with_capacity(lhs.hs.len() + rhs.hs.len());
    for h in &lhs.hs {
        let n = Vector4::new(h.n.x, h.n.y, 0.0, 0.0);
        hs.push(Hs4::new(n, h.c));
    }
    for h in &rhs.hs {
        let n = Vector4::new(0.0, 0.0, h.n.x, h.n.y);
        hs.push(Hs4::new(n, h.c));
    }
    Poly4::from_h(hs)
}

/// Specification of a 2D regular polygon.
#[derive(Clone, Debug)]
pub struct RegularPolygonSpec {
    pub sides: u32,
    pub rotation: f64,
    pub scale: f64,
}

impl RegularPolygonSpec {
    pub fn new(sides: u32, rotation: f64, scale: f64) -> Result<Self, GeneratorError> {
        if sides < 3 {
            return Err(GeneratorError::invalid(
                "regular polygon needs at least 3 sides",
            ));
        }
        if !scale.is_finite() || scale <= 0.0 {
            return Err(GeneratorError::invalid("scale must be finite and positive"));
        }
        if !rotation.is_finite() {
            return Err(GeneratorError::invalid("rotation must be finite"));
        }
        Ok(Self {
            sides,
            rotation,
            scale,
        })
    }

    fn vertices(&self) -> Vec<Vector2<f64>> {
        let mut verts = Vec::with_capacity(self.sides as usize);
        let angle_step = 2.0 * std::f64::consts::PI / self.sides as f64;
        for i in 0..self.sides {
            let theta = self.rotation + angle_step * i as f64;
            let x = self.scale * theta.cos();
            let y = self.scale * theta.sin();
            verts.push(Vector2::new(x, y));
        }
        verts
    }
}

/// Parameters for the regular polygon product enumerator.
#[derive(Clone, Debug)]
pub struct RegularProductEnumParams {
    pub factors_a: Vec<RegularPolygonSpec>,
    pub factors_b: Vec<RegularPolygonSpec>,
    pub max_pairs: Option<usize>,
}

impl RegularProductEnumParams {
    fn validate(&self) -> Result<(), GeneratorError> {
        if self.factors_a.is_empty() {
            return Err(GeneratorError::invalid(
                "need at least one polygon for factor A",
            ));
        }
        if self.factors_b.is_empty() {
            return Err(GeneratorError::invalid(
                "need at least one polygon for factor B",
            ));
        }
        if let Some(limit) = self.max_pairs {
            if limit == 0 {
                return Err(GeneratorError::invalid(
                    "max_pairs must be > 0 when specified",
                ));
            }
        }
        Ok(())
    }

    fn total_pairs(&self) -> usize {
        self.factors_a.len() * self.factors_b.len()
    }
}

/// Replay token storing the index pair for a deterministic tuple.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RegularProductReplay {
    pub index_a: usize,
    pub index_b: usize,
}

/// Enumerate lagrangian products of regular polygons.
pub struct RegularProductEnumerator {
    params: RegularProductEnumParams,
    next_linear_index: usize,
    yielded: usize,
}

impl RegularProductEnumerator {
    pub fn new(params: RegularProductEnumParams) -> Result<Self, GeneratorError> {
        params.validate()?;
        let total_pairs = params.total_pairs();
        if let Some(limit) = params.max_pairs {
            if limit > total_pairs {
                return Err(GeneratorError::invalid(
                    "max_pairs cannot exceed total pair count",
                ));
            }
        }
        Ok(Self {
            params,
            next_linear_index: 0,
            yielded: 0,
        })
    }

    fn build_poly(&self, replay: &RegularProductReplay) -> Result<Poly4, GeneratorError> {
        let spec_a = self
            .params
            .factors_a
            .get(replay.index_a)
            .ok_or_else(|| GeneratorError::invalid("factor A index out of range"))?;
        let spec_b = self
            .params
            .factors_b
            .get(replay.index_b)
            .ok_or_else(|| GeneratorError::invalid("factor B index out of range"))?;
        let verts_a = spec_a.vertices();
        let verts_b = spec_b.vertices();
        let mut verts = Vec::with_capacity(verts_a.len() * verts_b.len());
        for va in &verts_a {
            for vb in &verts_b {
                verts.push(Vector4::new(va.x, va.y, vb.x, vb.y));
            }
        }
        Ok(Poly4::from_v(verts))
    }
}

impl PolytopeGenerator4 for RegularProductEnumerator {
    type Params = RegularProductEnumParams;
    type Replay = RegularProductReplay;

    fn params(&self) -> &Self::Params {
        &self.params
    }

    fn generate_next(
        &mut self,
    ) -> Result<Option<PolytopeSample4<Self::Params, Self::Replay>>, GeneratorError> {
        if let Some(limit) = self.params.max_pairs {
            if self.yielded >= limit {
                return Ok(None);
            }
        }
        if self.next_linear_index >= self.params.total_pairs() {
            return Ok(None);
        }
        let len_b = self.params.factors_b.len();
        let index_a = self.next_linear_index / len_b;
        let index_b = self.next_linear_index % len_b;
        let replay = RegularProductReplay { index_a, index_b };
        let poly = self.build_poly(&replay)?;
        self.next_linear_index += 1;
        self.yielded += 1;
        Ok(Some(PolytopeSample4 {
            polytope: poly,
            params: self.params.clone(),
            replay,
        }))
    }

    fn regenerate(&self, replay: &Self::Replay) -> Result<Poly4, GeneratorError> {
        self.build_poly(replay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symmetric_halfspace_generator_replays() {
        let params = SymmetricHalfspaceParams {
            directions: 4,
            radius_min: 0.5,
            radius_max: 1.0,
            anisotropy: None,
        };
        let mut gen = SymmetricHalfspaceGenerator::new(params.clone(), 1234).unwrap();
        let sample = gen.generate_next().unwrap().unwrap();
        assert_eq!(sample.params.directions, params.directions);
        let replayed = gen.regenerate(&sample.replay).unwrap();
        assert_eq!(sample.polytope.h.len(), replayed.h.len());
        let mut poly = sample.polytope.clone();
        assert_eq!(poly.contains_origin(), Some(true));
        assert!(poly.is_convex());
    }

    #[test]
    fn regular_product_enumerator_iterates_and_limits() {
        let poly_a = RegularPolygonSpec::new(4, 0.0, 1.0).unwrap();
        let poly_b = RegularPolygonSpec::new(5, 0.1, 0.8).unwrap();
        let alt_b = RegularPolygonSpec::new(3, 0.2, 0.5).unwrap();
        let params = RegularProductEnumParams {
            factors_a: vec![poly_a.clone()],
            factors_b: vec![poly_b.clone(), alt_b.clone()],
            max_pairs: Some(1),
        };
        let mut gen = RegularProductEnumerator::new(params.clone()).unwrap();
        let first = gen.generate_next().unwrap().unwrap();
        assert_eq!(first.replay.index_a, 0);
        assert_eq!(first.replay.index_b, 0);
        assert!(gen.generate_next().unwrap().is_none());
        let replayed = gen.regenerate(&first.replay).unwrap();
        assert_eq!(first.polytope.v.len(), replayed.v.len());
    }

    #[test]
    fn mahler_product_generator_replays() {
        let params = MahlerProductParams {
            radial_cfg: RadialCfg {
                vertex_count: VertexCount::Uniform { min: 6, max: 8 },
                angle_jitter_frac: 0.2,
                radial_jitter: 0.15,
                base_radius: 1.0,
                random_phase: true,
            },
            bounds: Bounds2 {
                r_in_min: 0.2,
                r_out_max: 2.5,
            },
            max_attempts: 8,
        };
        let mut gen = MahlerProductGenerator::new(params.clone(), 2025).unwrap();
        let sample = gen.generate_next().unwrap().unwrap();
        let mut poly = sample.polytope.clone();
        assert_eq!(poly.contains_origin(), Some(true));
        let replayed = gen.regenerate(&sample.replay).unwrap();
        assert_eq!(replayed.h.len(), sample.polytope.h.len());
    }
}
