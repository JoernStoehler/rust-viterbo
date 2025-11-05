use nalgebra::{Matrix2, Vector2};

/// TH: VITERBO-2.1
/// VK: 00000000-0000-0000-0000-000000000000
/// Pre: `a` and `b` are 2D column vectors.
/// Post: returns the signed area of the parallelogram spanned by (a,b).
pub fn parallelogram_area(a: Vector2<f64>, b: Vector2<f64>) -> f64 {
    // determinant of [a b]
    let m = Matrix2::from_columns(&[a, b]);
    m.determinant()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::vector;
    use rand::{rngs::StdRng, Rng, SeedableRng};

    #[test]
    fn area_axis_aligned() {
        let a = vector![1.0, 0.0];
        let b = vector![0.0, 2.5];
        assert!((parallelogram_area(a, b) - 2.5).abs() < 1e-12);
    }

    #[test]
    fn area_randomized_seeded() {
        let mut rng = StdRng::seed_from_u64(42);
        let a = Vector2::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));
        let b = Vector2::new(rng.gen_range(-2.0..2.0), rng.gen_range(-2.0..2.0));
        // area equals |a_x b_y - a_y b_x|
        let expected = a.x * b.y - a.y * b.x;
        assert!((parallelogram_area(a, b) - expected).abs() < 1e-12);
    }
}
