use std::{collections::HashSet, io::Write, time::Duration};

use parking_lot::RwLock;
use rayon::prelude::*;

use basic::{
    gameplay::{Board, Shape},
    piece_placer::PiecePlacer,
};

use crate::counter::Counter;

pub fn compute() -> Vec<Board> {
    let mut possible: HashSet<Board> = HashSet::new();
    possible.insert(Board(0xFFFFFFFFFF));

    for stage in (0..10).into_iter().rev() {
        let mino_count = stage * 4;
        let count_total = Counter::zero();
        let count_success = Counter::zero();
        let total: u64 = 0x10000000000;

        let next_stage = StartAnywhereStage::empty();

        crossbeam::scope(|s| {
            s.spawn(|_| loop {
                let counted_total = count_total.get();
                let counted_success = count_success.get();
                eprint!(
                    "\r{:>13} / {:>13} ({:>6.2}%) -- {:>13}",
                    counted_total,
                    total,
                    (counted_total as f64) / (total as f64) * 100.,
                    counted_success,
                );
                std::io::stderr().flush().unwrap();
                if counted_total == total as u64 {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            });

            (0..total).into_par_iter().for_each(|b| {
                let board = Board(b);
                count_total.increment();

                if board.0.count_ones() != mino_count
                    || is_invalid(board)
                    || board.has_isolated_cell()
                    || board.has_imbalanced_split()
                {
                    return;
                }

                for shape in Shape::ALL {
                    for (_, new_board) in PiecePlacer::new(board, shape) {
                        if possible.contains(&new_board) {
                            next_stage.insert(board);
                            count_success.increment();
                            return;
                        }
                    }
                }
            });
        })
        .unwrap();

        next_stage.collect(&mut possible);
        eprintln!("  iteration {} complete", 10 - stage);
    }

    let mut all_boards = Vec::new();

    eprintln!("collecting boards...");
    all_boards.extend(possible.iter());
    eprintln!("sorting...");
    all_boards.par_sort_unstable();
    eprintln!("done");

    all_boards
}

fn is_invalid(board: Board) -> bool {
    #[derive(Eq, Ord, PartialEq, PartialOrd)]
    enum State {
        Full,
        Mixed,
        Empty,
        Invalid,
    }

    use State::*;

    fn step(state: State, line: u64) -> State {
        let line = match line & 0b1111111111 {
            0b0000000000 => Empty,
            0b1111111111 => Full,
            _ => Mixed,
        };

        if state <= line {
            line
        } else {
            Invalid
        }
    }

    let state = step(Full, board.0 >> 0);
    let state = step(state, board.0 >> 10);
    let state = step(state, board.0 >> 20);
    let state = step(state, board.0 >> 30);

    state == Invalid
}

pub struct StartAnywhereStage(Vec<RwLock<Vec<Board>>>);

impl StartAnywhereStage {
    pub fn empty() -> StartAnywhereStage {
        let mut locks = Vec::new();
        locks.resize_with(num_cpus::get(), || RwLock::new(Vec::new()));
        StartAnywhereStage(locks)
    }

    pub fn insert(&self, board: Board) {
        let shard = &self.0[rayon::current_thread_index().unwrap()];
        shard.write().push(board);
    }

    pub fn collect<E: Extend<Board>>(&self, into: &mut E) {
        for shard in &self.0 {
            into.extend(shard.read().iter().copied());
        }
    }
}
