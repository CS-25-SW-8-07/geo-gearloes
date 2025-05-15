pub mod car;
pub mod config;
pub mod sim;
pub mod widgets;

use config::{SimConfig, config_module};
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use sim::Sim;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn start_sim(py: Python<'_>, config: Bound<'_, SimConfig>) -> PyResult<()> {
    let config = config.unbind();
    py.allow_threads(|| {
        Sim(config)
            .run()
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))
    })
}

/// A Python module implemented in Rust.
#[pymodule]
fn pysim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    config_module(m)?;
    m.add_function(wrap_pyfunction!(start_sim, m)?)?;
    Ok(())
}
