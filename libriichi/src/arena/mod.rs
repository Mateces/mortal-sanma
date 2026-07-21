mod board;
mod game;
mod one_vs_two;
mod result;
mod three_way;

pub use board::Board;
pub use one_vs_two::OneVsTwo;
pub use result::GameResult;
pub use three_way::ThreeWay;

use crate::py_helper::add_submodule;

use pyo3::prelude::*;

pub(crate) fn register_module(
    py: Python<'_>,
    prefix: &str,
    super_mod: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let m = PyModule::new(py, "arena")?;
    m.add_class::<OneVsTwo>()?;
    m.add_class::<ThreeWay>()?;
    add_submodule(py, prefix, super_mod, &m)
}
