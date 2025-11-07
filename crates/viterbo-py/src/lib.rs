use nalgebra::Vector2;
use pyo3::prelude::*;

/// Compute signed area of the parallelogram spanned by a and b.
#[pyfunction]
fn parallelogram_area(a: (f64, f64), b: (f64, f64)) -> f64 {
    let va = Vector2::new(a.0, a.1);
    let vb = Vector2::new(b.0, b.1);
    viterbo::parallelogram_area(va, vb)
}

/// TODO stub: 2D polygon sampler (radial jitter) binding.
///
/// The Rust API lives at `viterbo::geom2::rand`. Python binding deferred per ticket scope.
#[pyfunction]
fn polygon_sampler_todo() -> PyResult<()> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TODO: Bindings for viterbo.geom2.rand.draw_polygon_radial are deferred.",
    ))
}

/// TODO stub: 2D polar polytope binding.
#[pyfunction]
fn polygon_polar_todo() -> PyResult<()> {
    Err(pyo3::exceptions::PyNotImplementedError::new_err(
        "TODO: Bindings for viterbo.geom2.rand.polar are deferred.",
    ))
}

#[pymodule]
fn viterbo_native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parallelogram_area, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_sampler_todo, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_polar_todo, m)?)?;
    Ok(())
}
