//! Small utilities: combinations and geometric dedup/quantization helpers.

use nalgebra::Vector4;

/// k-combinations of items (lexicographic).
pub(crate) fn combinations<T: Copy>(items: &[T], k: usize) -> Vec<Vec<T>> {
    let n = items.len();
    if k > n || k == 0 {
        return Vec::new();
    }
    let mut idxs: Vec<usize> = (0..k).collect();
    let mut out = Vec::new();
    loop {
        out.push(idxs.iter().map(|&i| items[i]).collect());
        // next combination
        let mut i = k;
        while i > 0 {
            i -= 1;
            if idxs[i] != i + n - k {
                idxs[i] += 1;
                for j in i + 1..k {
                    idxs[j] = idxs[j - 1] + 1;
                }
                break;
            }
        }
        if i == 0 && idxs[0] == n - k {
            break;
        }
        if i == 0 && idxs[0] > n - k {
            break;
        }
        if idxs[0] == n - k && idxs[k - 1] == n - 1 {
            break;
        }
    }
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

