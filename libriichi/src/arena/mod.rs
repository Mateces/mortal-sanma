mod board;
mod game;
mod result;

pub use board::Board;
pub use result::GameResult;

use crate::py_helper::add_submodule;

use pyo3::prelude::*;

pub(crate) fn register_module(
    py: Python<'_>,
    prefix: &str,
    super_mod: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let m = PyModule::new(py, "arena")?;
    add_submodule(py, prefix, super_mod, &m)
}
