use std::{collections::HashSet, io::Write, time::Duration};

use rayon::prelude::*;

use basic::{
    gameplay::{Board, Shape},
    piece_placer::PiecePlacer,
};

use super::Stage;
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
                    || board.shift_full() != board
                    || board.has_isolated_cell()
                    || board.has_imbalanced_split()
                {
                    return;
                }

                for shape in Shape::ALL {
                    for (_, new_board) in PiecePlacer::new(board, shape) {
                        if possible.contains(&new_board) {
                            next_stage.0.lock_subset(board).insert(board, ());
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

pub struct StartAnywhereStage(pub Stage<()>);

impl StartAnywhereStage {
    pub fn empty() -> StartAnywhereStage {
        StartAnywhereStage(Stage::empty())
    }

    pub fn collect(&self, boards: &mut HashSet<Board>) {
        for subset in self.0.lock_all().0 {
            for (&board, _) in subset.iter() {
                boards.insert(board);
            }
        }
    }
}
