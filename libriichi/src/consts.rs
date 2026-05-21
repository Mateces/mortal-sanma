use crate::py_helper::add_submodule;

use pyo3::prelude::*;

/// Number of players in sanma.
pub const NUM_PLAYERS: usize = 3;

pub const MAX_VERSION: u32 = 5;

pub const ACTION_SPACE: usize = 37 // discard | kan (choice)
                              + 1  // riichi
                              + 1  // nukidora (抜きドラ)
                              + 1  // pon
                              + 1  // kan (decide)
                              + 1  // agari
                              + 1  // ryukyoku
                              + 1; // pass
// = 44 (chi removed, nukidora added)

/// GRP input size: [grand_kyoku, honba, kyotaku, s0, s1, s2] = 6.
pub const GRP_SIZE: usize = 6;

#[pyfunction]
#[inline]
pub const fn obs_shape(version: u32) -> (usize, usize) {
    match version {
        // Legacy 4-player versions kept for reference (not usable in sanma)
        1 => (938, 34),
        2 => (942, 34),
        3 => (934, 34),
        4 => (1012, 34),
        // Sanma version
        5 => (775, 34),
        _ => unreachable!(),
    }
}

#[pyfunction]
#[inline]
pub const fn oracle_obs_shape(version: u32) -> (usize, usize) {
    match version {
        1 => (211, 34),
        2 | 3 | 4 => (217, 34),
        5 => (145, 34),
        _ => unreachable!(),
    }
}

pub(crate) fn register_module(
    py: Python<'_>,
    prefix: &str,
    super_mod: &Bound<'_, PyModule>,
) -> PyResult<()> {
    let m = PyModule::new(py, "consts")?;
    m.add_function(wrap_pyfunction!(obs_shape, &m)?)?;
    m.add_function(wrap_pyfunction!(oracle_obs_shape, &m)?)?;
    m.add("MAX_VERSION", MAX_VERSION)?;
    m.add("ACTION_SPACE", ACTION_SPACE)?;
    m.add("GRP_SIZE", GRP_SIZE)?;
    m.add("NUM_PLAYERS", NUM_PLAYERS)?;
    add_submodule(py, prefix, super_mod, &m)
}
