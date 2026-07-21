use super::game::{BatchGame, Index};
use super::result::GameResult;
use crate::agent::new_py_agent;

use std::fs::{self, File};
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use flate2::Compression;
use flate2::read::GzEncoder;
use indicatif::{ParallelProgressIterator, ProgressBar, ProgressStyle};
use pyo3::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;

const PERMS: [[u8; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

#[pyclass]
#[derive(Clone, Default)]
pub struct ThreeWay {
    pub disable_progress_bar: bool,
    pub log_dir: Option<String>,
}

#[pymethods]
impl ThreeWay {
    #[new]
    #[pyo3(signature = (*, disable_progress_bar=false, log_dir=None))]
    const fn new(disable_progress_bar: bool, log_dir: Option<String>) -> Self {
        Self {
            disable_progress_bar,
            log_dir,
        }
    }

    #[pyo3(signature = (engine0, engine1, engine2, seed_start, seed_count))]
    pub fn py_vs_py_vs_py(
        &self,
        engine0: PyObject,
        engine1: PyObject,
        engine2: PyObject,
        seed_start: (u64, u64),
        seed_count: u64,
        py: Python<'_>,
    ) -> Result<Vec<(u64, u64, [String; 3], [i32; 3], [u8; 3], [u8; 3])>> {
        py.allow_threads(move || {
            let results =
                self.run_batch(engine0, engine1, engine2, seed_start, seed_count)?;
            let records = results
                .into_iter()
                .map(|(r, seat_to_engine)| {
                    let rankings = r.rankings();
                    (
                        r.seed.0,
                        r.seed.1,
                        r.names,
                        r.scores,
                        rankings.rank_by_player,
                        seat_to_engine,
                    )
                })
                .collect();
            Ok(records)
        })
    }
}

impl ThreeWay {
    pub fn run_batch(
        &self,
        engine0: PyObject,
        engine1: PyObject,
        engine2: PyObject,
        seed_start: (u64, u64),
        seed_count: u64,
    ) -> Result<Vec<(GameResult, [u8; 3])>> {
        if let Some(dir) = &self.log_dir {
            fs::create_dir_all(dir)?;
        }

        log::info!(
            "3-way arena: seeds [{}, {}) w/ {:#x}, {} games",
            seed_start.0,
            seed_start.0 + seed_count,
            seed_start.1,
            seed_count,
        );

        let mut rng = ChaCha8Rng::seed_from_u64(
            seed_start.0.wrapping_mul(0x9E3779B97F4A7C15) ^ seed_start.1,
        );

        let seeds: Vec<(u64, u64)> = (seed_start.0..seed_start.0 + seed_count)
            .map(|s| (s, seed_start.1))
            .collect();

        let mut pids: [Vec<u8>; 3] = Default::default();
        let mut indexes: Vec<[Index; 3]> = Vec::with_capacity(seeds.len());
        let mut assignments: Vec<[u8; 3]> = Vec::with_capacity(seeds.len());

        for _ in 0..seed_count {
            let perm = PERMS[rng.gen_range(0..6usize)];
            let mut idxs = [Index::default(); 3];
            for seat in 0..3usize {
                let engine = perm[seat] as usize;
                let pid_idx = pids[engine].len();
                pids[engine].push(seat as u8);
                idxs[seat] = Index {
                    agent_idx: engine,
                    player_id_idx: pid_idx,
                };
            }
            indexes.push(idxs);
            assignments.push(perm);
        }

        let mut agents = [
            new_py_agent(engine0, &pids[0])?,
            new_py_agent(engine1, &pids[1])?,
            new_py_agent(engine2, &pids[2])?,
        ];

        let batch_game = BatchGame::tenhou_hanchan(self.disable_progress_bar);
        let results = batch_game.run(&mut agents, &indexes, &seeds)?;

        if let Some(dir) = &self.log_dir {
            log::info!("dumping 3-way game logs");
            let bar = if self.disable_progress_bar {
                ProgressBar::hidden()
            } else {
                ProgressBar::new(seeds.len() as u64)
            };
            const TEMPLATE: &str = "[{elapsed_precise}] [{wide_bar}] {pos}/{len} {percent:>3}%";
            bar.set_style(ProgressStyle::with_template(TEMPLATE)?.progress_chars("#-"));
            bar.enable_steady_tick(Duration::from_millis(150));

            results
                .par_iter()
                .progress_with(bar)
                .enumerate()
                .try_for_each(|(i, game_result)| {
                    let (seed, key) = game_result.seed;
                    let filename: PathBuf =
                        [dir, &format!("{seed}_{key}_{i:04}.json.gz")].iter().collect();
                    let log = game_result.dump_json_log()?;
                    let mut comp = GzEncoder::new(log.as_bytes(), Compression::best());
                    let mut f = File::create(filename)?;
                    io::copy(&mut comp, &mut f)?;
                    anyhow::Ok(())
                })?;
        }

        Ok(results.into_iter().zip(assignments).collect())
    }
}
