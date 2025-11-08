//! PyO3 bindings for selected `viterbo` functions.
//!
//! Notes
//! - Keep bindings thin and predictable; conversions use simple tuples/NumPy
//!   in higher-level wrappers in `src/viterbo/rust/`.
//! - Most native functionality remains in Rust (`viterbo` crate). We only bind
//!   pieces that are proven hot or ergonomically valuable for Python callers.

use nalgebra::{Vector2, Vector4};
use pyo3::prelude::*;
use viterbo::geom4::{volume4, Hs4, Poly4};

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

/// Compute the 4D volume of a convex polytope from its half-spaces.
#[pyfunction]
fn poly4_volume_from_halfspaces(hs: Vec<((f64, f64, f64, f64), f64)>) -> PyResult<f64> {
    if hs.len() < 5 {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "need at least 5 half-spaces for a bounded 4D polytope",
        ));
    }
    let mut poly = Poly4::from_h(
        hs.into_iter()
            .map(|(normal, c)| {
                let n = Vector4::new(normal.0, normal.1, normal.2, normal.3);
                Hs4::new(n, c)
            })
            .collect(),
    );
    volume4(&mut poly)
        .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
}

#[pymodule]
fn viterbo_native(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parallelogram_area, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_sampler_todo, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_polar_todo, m)?)?;
    m.add_function(wrap_pyfunction!(poly4_volume_from_halfspaces, m)?)?;
    Ok(())
}
