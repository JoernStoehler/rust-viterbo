//! Random convex polygons in 2D (radial jitter + replay tokens).
//!
//! Purpose
//! - Provide a small, deterministic sampler for convex polygons used by the 2D Mahler-product experiments. The generator is parameterizable, reproducible, and returns strict H-rep (`Poly2`), ready for push-forwards and HPI.
//!
//! Model
//! - Start from `n` equally spaced angles on [0, 2π), add bounded angular and
//!   radial jitter, build the convex hull, then recenter/scale as requested.
//! - Determinism uses a replay token `(seed, index)` mixed into a single RNG.
//!
//! References
//! - TH: docs/src/thesis/geom2d_polytopes.md (section “Random 2D polygons”)
//! - Code cross-refs: `Poly2`, `from_points_convex_hull_strict`

use super::{ordered::HalfspaceIntersection, ordered::Poly2, Aff2};
use nalgebra::Vector2;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Vertex count distribution.
#[derive(Clone, Copy, Debug)]
pub enum VertexCount {
    Fixed(usize),
    Uniform { min: usize, max: usize },
}
impl VertexCount {
    fn sample<R: Rng>(&self, rng: &mut R) -> usize {
        match *self {
            VertexCount::Fixed(n) => n.max(3),
            VertexCount::Uniform { min, max } => {
                let lo = min.max(3);
                let hi = max.max(lo);
                rng.gen_range(lo..=hi)
            }
        }
    }
}

/// Radial-jitter sampler configuration.
#[derive(Clone, Copy, Debug)]
pub struct RadialCfg {
    pub vertex_count: VertexCount,
    /// Angular jitter as a fraction of the base spacing Δ=2π/n. Clamped to [0, 0.49].
    pub angle_jitter_frac: f64,
    /// Radial jitter (relative amplitude). Radii = `base_radius * (1 + u)`, with `u∈[-radial_jitter, radial_jitter]`.
    pub radial_jitter: f64,
    /// Base radius before recenter/rescale.
    pub base_radius: f64,
    /// Random global phase in [0, 2π)?
    pub random_phase: bool,
}
impl Default for RadialCfg {
    fn default() -> Self {
        Self {
            vertex_count: VertexCount::Fixed(12),
            angle_jitter_frac: 0.3,
            radial_jitter: 0.25,
            base_radius: 1.0,
            random_phase: true,
        }
    }
}

/// Bounds for recenter/rescale around the origin.
#[derive(Clone, Copy, Debug)]
pub struct Bounds2 {
    /// Minimum inradius (distance from origin to closest edge). If <=0, ignored.
    pub r_in_min: f64,
    /// Maximum outradius (max vertex norm). If <=0, ignored.
    pub r_out_max: f64,
}

/// Replay token to make draws reproducible and indexable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplayToken {
    pub seed: u64,
    pub index: u64,
}
impl ReplayToken {
    #[inline]
    fn to_std_rng(self) -> StdRng {
        // SplitMix64-style mixing, cheap and stable.
        fn mix(mut x: u64) -> u64 {
            x ^= x >> 30;
            x = x.wrapping_mul(0xbf58476d1ce4e5b9);
            x ^= x >> 27;
            x = x.wrapping_mul(0x94d049bb133111eb);
            x ^ (x >> 31)
        }
        let k = mix(self.seed ^ mix(self.index.wrapping_add(0x9e3779b97f4a7c15)));
        StdRng::seed_from_u64(k)
    }
}

/// Draw a random convex polygon (strict H-rep) via radial jitter + convex hull.
///
/// Notes
/// - The polygon is near the origin before recentering, but origin containment is only guaranteed if you subsequently call `recenter_rescale`.
pub fn draw_polygon_radial(cfg: RadialCfg, tok: ReplayToken) -> Option<Poly2> {
    let mut rng = tok.to_std_rng();
    let n = cfg.vertex_count.sample(&mut rng).max(3);
    let aj = cfg.angle_jitter_frac.clamp(0.0, 0.49);
    let rj = cfg.radial_jitter.max(0.0);
    let r0 = cfg.base_radius.max(1e-9);
    let delta = 2.0 * std::f64::consts::PI / (n as f64);
    let phase = if cfg.random_phase {
        rng.gen::<f64>() * 2.0 * std::f64::consts::PI
    } else {
        0.0
    };
    let mut angles: Vec<f64> = (0..n)
        .map(|k| {
            let base = phase + (k as f64) * delta;
            let jitter = (rng.gen::<f64>() * 2.0 - 1.0) * aj * delta;
            base + jitter
        })
        .collect();
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pts: Vec<Vector2<f64>> = angles
        .into_iter()
        .map(|th| {
            let u = (rng.gen::<f64>() * 2.0 - 1.0) * rj;
            let r = (1.0 + u).max(1e-6) * r0;
            Vector2::new(th.cos() * r, th.sin() * r)
        })
        .collect();
    super::util::from_points_convex_hull_strict(&pts)
}

/// Translate to origin’s area-centroid and scale to meet `Bounds2`, if consistent.
///
/// Returns `(poly, r_in, r_out)`. If both bounds are set and inconsistent, returns `None`.
pub fn recenter_rescale(poly: &Poly2, bounds: Bounds2) -> Option<(Poly2, f64, f64)> {
    let verts = match poly.halfspace_intersection() {
        HalfspaceIntersection::Bounded(v) => v,
        _ => return None,
    };
    let c = polygon_area_centroid(&verts)?;
    // Translate by -centroid.
    let translated = poly.push_forward(&Aff2 {
        m: nalgebra::Matrix2::identity(),
        t: -c,
    })?;
    let verts_t = match translated.halfspace_intersection() {
        HalfspaceIntersection::Bounded(v) => v,
        _ => return None,
    };
    let r_out0 = verts_t.iter().map(|p| p.norm()).fold(0.0, f64::max);
    let r_in0 = translated
        .hs
        .iter()
        .map(|h| h.c)
        .fold(f64::INFINITY, f64::min);
    let mut s_min = 0.0;
    let mut s_max = f64::INFINITY;
    if bounds.r_in_min > 0.0 {
        if r_in0 <= 0.0 {
            // If the origin isn’t inside after centroid shift, we can’t enforce r_in_min by scaling.
            return None;
        }
        s_min = (bounds.r_in_min / r_in0).max(s_min);
    }
    if bounds.r_out_max > 0.0 {
        if r_out0 <= 0.0 {
            return None;
        }
        s_max = (bounds.r_out_max / r_out0).min(s_max);
    }
    if s_min > s_max {
        return None; // inconsistent bounds
    }
    // Prefer no-op if possible, else pick s_min (smallest that satisfies inradius), clamped by s_max.
    let s = if 1.0 >= s_min && 1.0 <= s_max {
        1.0
    } else {
        s_min.clamp(1e-12, s_max)
    };
    let scaled = translated.push_forward(&Aff2 {
        m: nalgebra::Matrix2::identity() * s,
        t: Vector2::zeros(),
    })?;
    let r_out = r_out0 * s;
    let r_in = r_in0 * s;
    Some((scaled, r_in, r_out))
}

/// Polar polytope K° (H-representation) from strict H-rep `K = { x : n_i·x <= c_i }`.
///
/// Preconditions
/// - `n_i` are unit normals (as in `Poly2`), `c_i > 0` for origin containment.
///
/// Construction
/// - Vertices of K° are `{ n_i / c_i }`. We compute the hull and return H-rep.
pub fn polar(poly: &Poly2) -> Option<Poly2> {
    if poly.hs.is_empty() {
        return None;
    }
    let mut pts: Vec<Vector2<f64>> = Vec::with_capacity(poly.hs.len());
    for h in &poly.hs {
        if !(h.c.is_finite()) || h.c <= 0.0 {
            return None; // origin must be in the interior
        }
        let p = h.n / h.c;
        pts.push(p);
    }
    super::util::from_points_convex_hull_strict(&pts)
}

/// Compute polygon area centroid (assumes vertices in CCW order, non-degenerate).
fn polygon_area_centroid(verts: &[Vector2<f64>]) -> Option<Vector2<f64>> {
    if verts.len() < 3 {
        return None;
    }
    let mut a: f64 = 0.0;
    let mut cx: f64 = 0.0;
    let mut cy: f64 = 0.0;
    for i in 0..verts.len() {
        let p = verts[i];
        let q = verts[(i + 1) % verts.len()];
        let cross = p.x * q.y - q.x * p.y;
        a += cross;
        cx += (p.x + q.x) * cross;
        cy += (p.y + q.y) * cross;
    }
    a *= 0.5;
    if a.abs() < 1e-18 {
        return None;
    }
    Some(Vector2::new(cx / (6.0 * a), cy / (6.0 * a)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom2::from_points_convex_hull_strict;

    #[test]
    fn reproducible_draw() {
        let cfg = RadialCfg {
            vertex_count: VertexCount::Fixed(10),
            angle_jitter_frac: 0.2,
            radial_jitter: 0.1,
            base_radius: 1.0,
            random_phase: true,
        };
        let tok = ReplayToken { seed: 42, index: 7 };
        let p1 = draw_polygon_radial(cfg, tok).expect("poly");
        let p2 = draw_polygon_radial(cfg, tok).expect("poly");
        assert_eq!(p1.hs.len(), p2.hs.len());
        for (a, b) in p1.hs.iter().zip(p2.hs.iter()) {
            assert!((a.n - b.n).norm() < 1e-12);
            assert!((a.c - b.c).abs() < 1e-12);
        }
    }

    #[test]
    fn recenter_and_bounds() {
        let cfg = RadialCfg::default();
        let tok = ReplayToken {
            seed: 1,
            index: 123,
        };
        let p = draw_polygon_radial(cfg, tok).unwrap();
        let (q, r_in, r_out) = recenter_rescale(
            &p,
            Bounds2 {
                r_in_min: 0.2,
                r_out_max: 2.0,
            },
        )
        .unwrap();
        assert!(r_in >= 0.2 - 1e-12);
        assert!(r_out <= 2.0 + 1e-12);
        // Origin must be inside after recenter.
        assert!(q.hs.iter().all(|h| h.c > 0.0));
    }

    #[test]
    fn polar_and_bipolar() {
        // Start from a simple square around origin
        let points = vec![
            Vector2::new(1.0, 1.0),
            Vector2::new(1.0, -1.0),
            Vector2::new(-1.0, -1.0),
            Vector2::new(-1.0, 1.0),
        ];
        let p0 = from_points_convex_hull_strict(&points).unwrap();
        // Ensure strict origin containment via mild scale
        let (p, _, _) = recenter_rescale(
            &p0,
            Bounds2 {
                r_in_min: 0.5,
                r_out_max: 2.0,
            },
        )
        .unwrap();
        let q = polar(&p).expect("polar");
        let qq = polar(&q).expect("double polar");
        // Mutual containment within small slack
        let eps = 1e-9;
        // All vertices of qq inside p
        if let HalfspaceIntersection::Bounded(vs) = qq.halfspace_intersection() {
            for v in vs {
                assert!(p.contains_eps(v, eps));
            }
        } else {
            panic!("qq expected bounded");
        }
        // All vertices of p inside qq
        if let HalfspaceIntersection::Bounded(vs) = p.halfspace_intersection() {
            for v in vs {
                assert!(qq.contains_eps(v, eps));
            }
        } else {
            panic!("p expected bounded");
        }
    }
}
