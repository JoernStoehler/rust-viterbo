//! Small utilities: combinations and geometric dedup/quantization helpers.

use nalgebra::Vector4;

/// k-combinations of items (lexicographic).
pub(crate) fn combinations<T: Copy>(items: &[T], k: usize) -> Vec<Vec<T>> {
    fn helper<T: Copy>(
        items: &[T],
        start: usize,
        k: usize,
        current: &mut Vec<T>,
        out: &mut Vec<Vec<T>>,
    ) {
        if k == 0 {
            out.push(current.clone());
            return;
        }
        for i in start..=items.len() - k {
            current.push(items[i]);
            helper(items, i + 1, k - 1, current, out);
            current.pop();
        }
    }

    if k == 0 || k > items.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut current = Vec::with_capacity(k);
    helper(items, 0, k, &mut current, &mut out);
    out
}

pub(crate) fn dedup_points_in_place(points: &mut Vec<Vector4<f64>>, tol: f64) {
    if points.len() < 2 {
        return;
    }
    points.sort_by(|a, b| {
        a[0].partial_cmp(&b[0])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a[1].partial_cmp(&b[1]).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a[2].partial_cmp(&b[2]).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a[3].partial_cmp(&b[3]).unwrap_or(std::cmp::Ordering::Equal))
    });
    points.dedup_by(|a, b| (*a - *b).norm() < tol);
}

pub(crate) fn quantize4(v: Vector4<f64>, tol: f64) -> (i64, i64, i64, i64) {
    let s = 1.0 / tol;
    (
        (v[0] * s).round() as i64,
        (v[1] * s).round() as i64,
        (v[2] * s).round() as i64,
        (v[3] * s).round() as i64,
    )
}
pub(crate) fn quantize5(n: Vector4<f64>, c: f64, tol: f64) -> (i64, i64, i64, i64, i64) {
    let (x, y, z, w) = quantize4(n, tol);
    let s = 1.0 / tol;
    (x, y, z, w, (c * s).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::combinations;

    #[test]
    fn combos_cover_pairs() {
        let items = vec![0, 1, 2, 3];
        let combos = combinations(&items, 2);
        assert_eq!(combos.len(), 6);
        assert!(combos.contains(&vec![2, 3]));
    }
}
