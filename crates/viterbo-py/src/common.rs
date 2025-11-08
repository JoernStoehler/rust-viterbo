use nalgebra::Vector4;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use viterbo::geom4::{Hs4, Poly4, VolumeError};

pub fn poly4_from_py_halfspaces(
    hs: Vec<((f64, f64, f64, f64), f64)>,
) -> PyResult<Poly4> {
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
    poly.check_canonical()
        .map_err(|err| PyValueError::new_err(err))?;
    Ok(poly)
}

pub fn map_volume_err(err: VolumeError) -> PyErr {
    PyValueError::new_err(err.to_string())
}
