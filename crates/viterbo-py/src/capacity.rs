//! Oriented-edge capacity bindings.

use crate::common::poly4_from_py_halfspaces;
use pyo3::prelude::*;
use viterbo::oriented_edge::solve_with_defaults;

#[pyfunction]
pub fn poly4_capacity_ehz_from_halfspaces(
    hs: Vec<((f64, f64, f64, f64), f64)>,
) -> PyResult<Option<f64>> {
    let mut poly = poly4_from_py_halfspaces(hs)?;
    Ok(solve_with_defaults(&mut poly).map(|(c, _cycle)| c))
}

pub fn register(m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(
        poly4_capacity_ehz_from_halfspaces,
        m
    )?)?;
    Ok(())
}
