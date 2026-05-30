use super::game::{BatchGame, Index};
use super::result::GameResult;
use crate::agent::{BatchAgent, new_py_agent};
use std::fs::{self, File};
use std::io;
use std::iter;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use flate2::Compression;
use flate2::read::GzEncoder;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use rayon::prelude::*;

#[pyclass]
#[derive(Clone, Default)]
pub struct OneVsTwo {
    pub disable_progress_bar: bool,
    pub log_dir: Option<String>,
}

#[pymethods]
impl OneVsTwo {
    #[new]
    #[pyo3(signature = (*, disable_progress_bar=false, log_dir=None))]
    const fn new(disable_progress_bar: bool, log_dir: Option<String>) -> Self {
        Self {
            disable_progress_bar,
            log_dir,
        }
    }

    /// Returns the rankings of the challenger as `[#1, #2, #3]`.
    pub fn py_vs_py(
        &self,
        challenger: PyObject,
        champion: PyObject,
        seed_start: (u64, u64),
        seed_count: u64,
        py: Python<'_>,
    ) -> Result<[i32; 3]> {
        py.allow_threads(move || {
            let results = self.run_batch(
                |player_ids| new_py_agent(challenger, player_ids),
                |player_ids| new_py_agent(champion, player_ids),
                seed_start,
                seed_count,
            )?;

            let mut rankings = [0; 3];
            for (i, result) in results.iter().enumerate() {
                let rank = result.rankings().rank_by_player[i % 3];
                rankings[rank as usize] += 1;
            }
            Ok(rankings)
        })
    }

    /// Same as `py_vs_py`, but returns per-game records instead of aggregate
    /// rankings. Each record is
    /// `(seed, key, split_idx, [name0, name1, name2], [score0, score1, score2],
    ///  [rank0, rank1, rank2], challenger_seat)`,
    /// where `split_idx` is 0/1/2 (challenger seat A/B/C) and the score / rank
    /// arrays are indexed by table seat (0/1/2 = oya for kyoku 1/2/3).
    pub fn py_vs_py_detailed(
        &self,
        challenger: PyObject,
        champion: PyObject,
        seed_start: (u64, u64),
        seed_count: u64,
        py: Python<'_>,
    ) -> Result<Vec<(u64, u64, u8, [String; 3], [i32; 3], [u8; 3], u8)>> {
        py.allow_threads(move || {
            let results = self.run_batch(
                |player_ids| new_py_agent(challenger, player_ids),
                |player_ids| new_py_agent(champion, player_ids),
                seed_start,
                seed_count,
            )?;

            let records: Vec<_> = results
                .into_iter()
                .enumerate()
                .map(|(i, r)| {
                    let split_idx = (i % 3) as u8;
                    let challenger_seat = split_idx;
                    let rankings = r.rankings();
                    (
                        r.seed.0,
                        r.seed.1,
                        split_idx,
                        r.names,
                        r.scores,
                        rankings.rank_by_player,
                        challenger_seat,
                    )
                })
                .collect();
            Ok(records)
        })
    }
}

impl OneVsTwo {
    pub fn run_batch<C, M>(
        &self,
        new_challenger_agent: C,
        new_champion_agent: M,
        seed_start: (u64, u64),
        seed_count: u64,
    ) -> Result<Vec<GameResult>>
    where
        C: FnOnce(&[u8]) -> Result<Box<dyn BatchAgent>>,
        M: FnOnce(&[u8]) -> Result<Box<dyn BatchAgent>>,
    {
        if let Some(dir) = &self.log_dir {
            fs::create_dir_all(dir)?;
        }

        log::info!(
            "seed: [{}, {}) w/ {:#x}, start {} sets, {} hanchans",
            seed_start.0,
            seed_start.0 + seed_count,
            seed_start.1,
            seed_count,
            seed_count * 3,
        );

        // Each seed produces 3 hanchans (challenger sits in seats 0, 1, 2).
        let seeds: Vec<_> = (seed_start.0..seed_start.0 + seed_count)
            .flat_map(|seed| iter::repeat_n((seed, seed_start.1), 3))
            .collect();

        // The challenger plays one seat per split, cycling 0,1,2 across the
        // three splits of every seed.
        let challenger_player_ids: Vec<_> = (0..3).cycle().take(seed_count as usize * 3).collect();

        // Champion takes the remaining two seats per split.
        let champion_player_ids_per_seed = [
            1, 2, // split A: challenger=0
            0, 2, // split B: challenger=1
            0, 1, // split C: challenger=2
        ];
        let champion_player_ids: Vec<_> = champion_player_ids_per_seed
            .into_iter()
            .cycle()
            .take(seed_count as usize * champion_player_ids_per_seed.len())
            .collect();

        let mut agents = [
            new_challenger_agent(&challenger_player_ids)?,
            new_champion_agent(&champion_player_ids)?,
        ];
        let batch_game = BatchGame::tenhou_hanchan(self.disable_progress_bar);

        let mut challenger_idx = 0;
        let mut champion_idx = 0;
        let agent_idxs_per_seed = [
            [0, 1, 1], // split A: challenger seat 0
            [1, 0, 1], // split B: challenger seat 1
            [1, 1, 0], // split C: challenger seat 2
        ];
        let indexes: Vec<_> = agent_idxs_per_seed
            .into_iter()
            .cycle()
            .take(seed_count as usize * agent_idxs_per_seed.len())
            .map(|agent_idxs_per_split| {
                agent_idxs_per_split.map(|agent_idx| {
                    let player_id_idx = if agent_idx == 0 {
                        &mut challenger_idx
                    } else {
                        &mut champion_idx
                    };
                    let ret = Index {
                        agent_idx,
                        player_id_idx: *player_id_idx,
                    };
                    *player_id_idx += 1;
                    ret
                })
            })
            .collect();

        let results = batch_game.run(&mut agents, &indexes, &seeds)?;

        if let Some(dir) = &self.log_dir {
            log::info!("dumping game logs");

            let bar = if self.disable_progress_bar {
                ProgressBar::hidden()
            } else {
                ProgressBar::new(seed_count * 3)
            };
            const TEMPLATE: &str = "[{elapsed_precise}] [{wide_bar}] {pos}/{len} {percent:>3}%";
            bar.set_style(ProgressStyle::with_template(TEMPLATE)?.progress_chars("#-"));
            bar.enable_steady_tick(Duration::from_millis(150));

            results
                .par_iter()
                .progress_with(bar)
                .enumerate()
                .try_for_each(|(i, game_result)| {
                    let split_name = ["a", "b", "c"][i % 3];
                    let (seed, key) = game_result.seed;
                    let filename: PathBuf = [dir, &format!("{seed}_{key}_{split_name}.json.gz")]
                        .iter()
                        .collect();

                    let log = game_result.dump_json_log()?;
                    let mut comp = GzEncoder::new(log.as_bytes(), Compression::best());
                    let mut f = File::create(filename)?;
                    io::copy(&mut comp, &mut f)?;

                    anyhow::Ok(())
                })?;
        }

        Ok(results)
    }
}
