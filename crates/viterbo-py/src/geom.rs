//! Geometric helper bindings (kept separate so `lib.rs` stays tiny).

use nalgebra::{Vector2, Vector4};
use pyo3::exceptions::{PyNotImplementedError, PyValueError};
use pyo3::prelude::*;
use viterbo::geom4::{volume4, Hs4, Poly4};

#[pyfunction]
pub fn parallelogram_area(a: (f64, f64), b: (f64, f64)) -> f64 {
    let va = Vector2::new(a.0, a.1);
    let vb = Vector2::new(b.0, b.1);
    viterbo::parallelogram_area(va, vb)
}

#[pyfunction]
pub fn polygon_sampler_todo() -> PyResult<()> {
    Err(PyNotImplementedError::new_err(
        "TODO: Bindings for viterbo.geom2.rand.draw_polygon_radial are deferred.",
    ))
}

#[pyfunction]
pub fn polygon_polar_todo() -> PyResult<()> {
    Err(PyNotImplementedError::new_err(
        "TODO: Bindings for viterbo.geom2.rand.polar are deferred.",
    ))
}

#[pyfunction]
pub fn poly4_volume_from_halfspaces(
    hs: Vec<((f64, f64, f64, f64), f64)>,
) -> PyResult<f64> {
    if hs.len() < 5 {
        return Err(PyValueError::new_err(
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
    volume4(&mut poly).map_err(|err| PyValueError::new_err(err.to_string()))
}

pub fn register(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parallelogram_area, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_sampler_todo, m)?)?;
    m.add_function(wrap_pyfunction!(polygon_polar_todo, m)?)?;
    m.add_function(wrap_pyfunction!(poly4_volume_from_halfspaces, m)?)?;
    Ok(())
}
