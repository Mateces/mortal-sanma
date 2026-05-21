#[derive(Debug, Clone, Copy)]
pub struct Rankings {
    pub player_by_rank: [u8; 3],
    pub rank_by_player: [u8; 3],
}

impl Rankings {
    pub fn new(scores: [i32; 3]) -> Self {
        let mut player_by_rank = [0, 1, 2];
        player_by_rank.sort_by_key(|&i| -scores[i as usize]);

        let mut rank_by_player = [0; 3];
        for (rank, id) in player_by_rank.into_iter().enumerate() {
            rank_by_player[id as usize] = rank as u8;
        }

        Self {
            player_by_rank,
            rank_by_player,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn rankings() {
        let scores = [35000, 35000, 30000];
        let rk = Rankings::new(scores);
        assert_eq!(rk.player_by_rank, [0, 1, 2]);
        assert_eq!(rk.rank_by_player, [0, 1, 2]);

        let scores = [20000, 40000, 40000];
        let rk = Rankings::new(scores);
        assert_eq!(rk.player_by_rank, [1, 2, 0]);
        assert_eq!(rk.rank_by_player, [2, 0, 1]);
    }
}
