use nalgebra::Vector2;
use pyo3::prelude::*;

/// Compute signed area of the parallelogram spanned by a and b.
#[pyfunction]
fn parallelogram_area(a: (f64, f64), b: (f64, f64)) -> f64 {
    let va = Vector2::new(a.0, a.1);
    let vb = Vector2::new(b.0, b.1);
    viterbo::parallelogram_area(va, vb)
}

#[pymodule]
fn viterbo_native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parallelogram_area, m)?)?;
    Ok(())
}

