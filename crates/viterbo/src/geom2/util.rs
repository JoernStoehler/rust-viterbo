use nalgebra::Vector2;

use super::{ordered::Poly2, types::Hs2};

#[inline]
fn angle_of(n: Vector2<f64>) -> f64 {
    n.y.atan2(n.x)
}

#[inline]
fn canonicalize_unit(n: Vector2<f64>, c: f64) -> Option<(Vector2<f64>, f64)> {
    let norm = n.norm();
    if !(norm.is_finite()) || norm <= 0.0 {
        return None;
    }
    Some((n / norm, c / norm))
}

/// Andrew’s monotone chain convex hull (returns hull in CCW order).
fn convex_hull(points: &[Vector2<f64>]) -> Option<Vec<Vector2<f64>>> {
    if points.len() < 2 {
        return None;
    }
    let mut pts: Vec<_> = points.to_vec();
    pts.sort_by(|a, b| {
        match a.x.partial_cmp(&b.x).unwrap_or(std::cmp::Ordering::Equal) {
            std::cmp::Ordering::Equal => a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal),
            o => o,
        }
    });
    pts.dedup_by(|a, b| (*a - *b).norm() < 1e-12);
    if pts.len() < 2 {
        return None;
    }
    let mut lower: Vec<Vector2<f64>> = Vec::with_capacity(pts.len());
    for p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Vector2<f64>> = Vec::with_capacity(pts.len());
    for p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    let mut hull = lower;
    hull.extend(upper);
    Some(hull)
}

#[inline]
fn cross(a: Vector2<f64>, b: Vector2<f64>, c: Vector2<f64>) -> f64 {
    let ab = b - a;
    let ac = c - a;
    ab.x * ac.y - ab.y * ac.x
}

/// Build strict H-rep from points via convex hull and outward normals.
pub fn from_points_convex_hull_strict(points: &[Vector2<f64>]) -> Option<Poly2> {
    let hull = convex_hull(points)?;
    if hull.len() < 2 {
        return None;
    }
    let mut hs = Vec::with_capacity(hull.len());
    for k in 0..hull.len() {
        let p = hull[k];
        let q = hull[(k + 1) % hull.len()];
        let edge = q - p;
        // For CCW hull order, outward normal is 90° CW: (edge.y, -edge.x)
        let n = Vector2::new(edge.y, -edge.x);
        let c = n.dot(&p);
        if let Some((nn, cc)) = canonicalize_unit(n, c) {
            hs.push(Hs2::new(nn, cc));
        }
    }
    hs.sort_by(|a, b| {
        let aa = angle_of(a.n);
        let bb = angle_of(b.n);
        aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
    });
    // coalesce parallels
    let mut out: Vec<Hs2> = Vec::with_capacity(hs.len());
    for h in hs {
        if let Some(last) = out.last_mut() {
            if (last.n - h.n).norm() < 1e-9 {
                if h.c < last.c {
                    last.c = h.c;
                }
                continue;
            }
        }
        out.push(h);
    }
    Some(Poly2 { hs: out })
}
