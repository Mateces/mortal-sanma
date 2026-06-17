
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

pub const GRP_SIZE: usize = 6;

#[inline]
pub const fn obs_shape(version: u32) -> (usize, usize) {
    match version {
        1 => (938, 34),
        2 => (942, 34),
        3 => (934, 34),
        4 => (1012, 34),
        5 => (780, 34),
        _ => unreachable!(),
    }
}

#[inline]
pub const fn oracle_obs_shape(version: u32) -> (usize, usize) {
    match version {
        1 => (211, 34),
        2 | 3 | 4 => (217, 34),
        5 => (170, 34),
        _ => unreachable!(),
    }
}
