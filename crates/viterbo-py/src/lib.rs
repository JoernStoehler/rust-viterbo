//! PyO3 entrypoint for the native `viterbo` bindings.
//!
//! This file stays small on purpose: real bindings live in sibling modules
//! (`geom`, `rand4`, ...). That keeps future extensions reviewable and avoids
//! churn during rebases.

mod capacity;
mod common;
mod geom;
mod rand4;

use pyo3::prelude::*;

#[pymodule]
fn viterbo_native(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    geom::register(m)?;
    capacity::register(m)?;
    rand4::register(py, m)?;
    Ok(())
}
