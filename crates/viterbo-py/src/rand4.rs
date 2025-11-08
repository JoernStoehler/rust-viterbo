//! PyO3 bindings for the `viterbo::rand4` generator catalogue.
//!
//! The functions exposed here intentionally mirror the Python expectations:
//! - inputs are plain `dict`s / lists so configs round-trip through JSON;
//! - outputs are small dictionaries (`vertices`, `halfspaces`) that higher
//!   layers can convert to richer types without touching this module.

use nalgebra::Matrix4;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList};
use viterbo::geom2::rand::{
    Bounds2, RadialCfg, ReplayToken as Poly2ReplayToken, VertexCount,
};
use viterbo::geom4::Poly4;
use viterbo::rand4::{
    GeneratorError, MahlerProductGenerator, MahlerProductParams, RegularProductEnumParams,
    RegularProductEnumerator, RegularProductReplay, RegularPolygonSpec,
    SymmetricHalfspaceGenerator, SymmetricHalfspaceParams,
};

pub fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(rand4_symmetric_halfspace_sample, m)?)?;
    m.add_function(wrap_pyfunction!(rand4_mahler_product_sample, m)?)?;
    m.add_function(wrap_pyfunction!(rand4_regular_product_sample, m)?)?;
    // Keep the interpreter handle alive for potential future stateful sources.
    let _ = py;
    Ok(())
}

/// Sample a centrally symmetric random halfspace polytope.
#[pyfunction]
fn rand4_symmetric_halfspace_sample(
    py: Python<'_>,
    params: &PyDict,
    seed: u64,
) -> PyResult<PyObject> {
    let params_rs = symmetric_params_from_dict(params)?;
    let poly = SymmetricHalfspaceGenerator::generate_single(&params_rs, seed)
        .map_err(map_generator_error)?;
    poly4_to_py(py, poly)
}

/// Sample a Mahler product (K × K°) based on `(seed, index)` replay token.
#[pyfunction]
fn rand4_mahler_product_sample(
    py: Python<'_>,
    params: &PyDict,
    seed: u64,
    index: u64,
) -> PyResult<PyObject> {
    let params_rs = mahler_params_from_dict(params)?;
    let token = Poly2ReplayToken { seed, index };
    let poly =
        MahlerProductGenerator::sample_with_token(&params_rs, token).map_err(map_generator_error)?;
    poly4_to_py(py, poly)
}

/// Deterministically rebuild a lagrangian product of two regular polygons.
#[pyfunction]
fn rand4_regular_product_sample(
    py: Python<'_>,
    params: &PyDict,
    pair_index: usize,
) -> PyResult<Option<PyObject>> {
    let params_rs = regular_product_params_from_dict(params)?;
    if params_rs.factors_a.is_empty() || params_rs.factors_b.is_empty() {
        return Ok(None);
    }
    let total_pairs = params_rs.factors_a.len() * params_rs.factors_b.len();
    if pair_index >= total_pairs {
        return Ok(None);
    }
    let enumerator = RegularProductEnumerator::new(params_rs.clone()).map_err(map_generator_error)?;
    let len_b = params_rs.factors_b.len();
    let replay = RegularProductReplay {
        index_a: pair_index / len_b,
        index_b: pair_index % len_b,
    };
    let poly = enumerator.build_poly(&replay).map_err(map_generator_error)?;
    let obj = poly4_to_py(py, poly)?;
    Ok(Some(obj))
}

fn map_generator_error(err: GeneratorError) -> PyErr {
    PyValueError::new_err(err.to_string())
}

fn symmetric_params_from_dict(dict: &PyDict) -> PyResult<SymmetricHalfspaceParams> {
    let directions = get_required::<usize>(dict, "directions")?;
    let radius_min = get_required::<f64>(dict, "radius_min")?;
    let radius_max = get_required::<f64>(dict, "radius_max")?;
    let anisotropy = match dict.get_item("anisotropy")? {
        Some(value) if !value.is_none() => Some(matrix4_from_any(value)?),
        _ => None,
    };
    Ok(SymmetricHalfspaceParams {
        directions,
        radius_min,
        radius_max,
        anisotropy,
    })
}

fn mahler_params_from_dict(dict: &PyDict) -> PyResult<MahlerProductParams> {
    let mut params = MahlerProductParams::default();
    if let Some(radial_any) = dict.get_item("radial_cfg")? {
        let radial_dict = radial_any.downcast::<PyDict>()?;
        params.radial_cfg = radial_cfg_from_dict(radial_dict)?;
    }
    if let Some(bounds_any) = dict.get_item("bounds")? {
        let bounds_dict = bounds_any.downcast::<PyDict>()?;
        params.bounds = bounds_from_dict(bounds_dict)?;
    }
    if let Some(max_attempts) = dict.get_item("max_attempts")? {
        params.max_attempts = max_attempts.extract()?;
    }
    Ok(params)
}

fn regular_product_params_from_dict(dict: &PyDict) -> PyResult<RegularProductEnumParams> {
    let factors_a_any = dict
        .get_item("factors_a")?
        .ok_or_else(|| PyValueError::new_err("missing 'factors_a' list"))?;
    let factors_b_any = dict
        .get_item("factors_b")?
        .ok_or_else(|| PyValueError::new_err("missing 'factors_b' list"))?;
    let factors_a = polygon_specs_from_seq(factors_a_any, "factors_a")?;
    let factors_b = polygon_specs_from_seq(factors_b_any, "factors_b")?;
    let max_pairs = match dict.get_item("max_pairs")? {
        Some(value) if !value.is_none() => Some(value.extract::<usize>()?),
        _ => None,
    };
    Ok(RegularProductEnumParams {
        factors_a,
        factors_b,
        max_pairs,
    })
}

fn polygon_specs_from_seq(obj: &PyAny, label: &str) -> PyResult<Vec<RegularPolygonSpec>> {
    let iter = obj.iter()?;
    let mut specs = Vec::new();
    for (idx, item_res) in iter.enumerate() {
        let item = item_res?;
        let dict = item.downcast::<PyDict>().map_err(|_| {
            PyValueError::new_err(format!(
                "{label}[{idx}] must be a dict with 'sides', 'rotation', 'scale'"
            ))
        })?;
        let sides = get_required::<u32>(dict, "sides")?;
        let rotation = get_with_default::<f64>(dict, "rotation", 0.0)?;
        let scale = get_with_default::<f64>(dict, "scale", 1.0)?;
        let spec = RegularPolygonSpec::new(sides, rotation, scale).map_err(map_generator_error)?;
        specs.push(spec);
    }
    if specs.is_empty() {
        return Err(PyValueError::new_err(format!(
            "{label} must contain at least one polygon"
        )));
    }
    Ok(specs)
}

fn radial_cfg_from_dict(dict: &PyDict) -> PyResult<RadialCfg> {
    let mut cfg = RadialCfg::default();
    if let Some(vc_any) = dict.get_item("vertex_count")? {
        cfg.vertex_count = parse_vertex_count(vc_any)?;
    }
    if let Some(angle) = dict.get_item("angle_jitter_frac")? {
        cfg.angle_jitter_frac = angle.extract()?;
    }
    if let Some(radial) = dict.get_item("radial_jitter")? {
        cfg.radial_jitter = radial.extract()?;
    }
    if let Some(base) = dict.get_item("base_radius")? {
        cfg.base_radius = base.extract()?;
    }
    if let Some(phase) = dict.get_item("random_phase")? {
        cfg.random_phase = phase.extract()?;
    }
    Ok(cfg)
}

fn bounds_from_dict(dict: &PyDict) -> PyResult<Bounds2> {
    Ok(Bounds2 {
        r_in_min: get_with_default(dict, "r_in_min", 0.1)?,
        r_out_max: get_with_default(dict, "r_out_max", 2.0)?,
    })
}

fn parse_vertex_count(obj: &PyAny) -> PyResult<VertexCount> {
    if let Ok(fixed) = obj.extract::<usize>() {
        return Ok(VertexCount::Fixed(fixed));
    }
    let dict = obj.downcast::<PyDict>().map_err(|_| {
        PyValueError::new_err("vertex_count must be an int or {\"kind\": ...}")
    })?;
    let kind = get_required::<String>(dict, "kind")?;
    match kind.as_str() {
        "fixed" => {
            let value = get_required::<usize>(dict, "value")?;
            Ok(VertexCount::Fixed(value))
        }
        "uniform" => {
            let min = get_required::<usize>(dict, "min")?;
            let max = get_required::<usize>(dict, "max")?;
            Ok(VertexCount::Uniform { min, max })
        }
        other => Err(PyValueError::new_err(format!(
            "vertex_count.kind must be 'fixed' or 'uniform', got {other}"
        ))),
    }
}

fn matrix4_from_any(obj: &PyAny) -> PyResult<Matrix4<f64>> {
    let rows: Vec<Vec<f64>> = obj.extract()?;
    if rows.len() != 4 {
        return Err(PyValueError::new_err(
            "anisotropy matrices must have four rows",
        ));
    }
    let mut data = [0.0f64; 16];
    for (i, row) in rows.into_iter().enumerate() {
        if row.len() != 4 {
            return Err(PyValueError::new_err(
                "anisotropy matrices must have four columns",
            ));
        }
        for (j, value) in row.into_iter().enumerate() {
            data[i * 4 + j] = value;
        }
    }
    Ok(Matrix4::from_row_slice(&data))
}

fn poly4_to_py(py: Python<'_>, mut poly: Poly4) -> PyResult<PyObject> {
    poly.ensure_vertices_from_h();
    poly.ensure_halfspaces_from_v();

    let verts = PyList::empty(py);
    for v in &poly.v {
        verts.append(PyList::new(py, [v[0], v[1], v[2], v[3]]))?;
    }
    let halfspaces = PyList::empty(py);
    for h in &poly.h {
        halfspaces.append(PyList::new(py, [h.n[0], h.n[1], h.n[2], h.n[3], h.c]))?;
    }
    let dict = PyDict::new(py);
    dict.set_item("vertices", verts)?;
    dict.set_item("halfspaces", halfspaces)?;
    Ok(dict.into())
}

fn get_required<'py, T: FromPyObject<'py>>(dict: &'py PyDict, key: &str) -> PyResult<T> {
    match dict.get_item(key)? {
        Some(value) => value.extract(),
        None => Err(PyValueError::new_err(format!(
            "missing required key '{key}'"
        ))),
    }
}

fn get_with_default<'py, T>(dict: &'py PyDict, key: &str, default: T) -> PyResult<T>
where
    T: FromPyObject<'py>,
{
    match dict.get_item(key)? {
        Some(value) => value.extract(),
        None => Ok(default),
    }
}
