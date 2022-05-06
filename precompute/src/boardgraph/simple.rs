use std::{collections::HashSet, io::Write, time::Duration};

use rayon::prelude::*;
use smallvec::SmallVec;

use srs_4l::{
    gameplay::{Board, Shape},
    piece_placer::PiecePlacer,
};

use super::Stage;
use crate::counter::Counter;

pub struct SimpleGraph(pub Vec<SimpleStage>);

pub fn compute() -> Vec<Board> {
    let mut forward_stages = Vec::new();
    forward_stages.push(SimpleStage::new());

    for iter in 1..=10 {
        let counter = Counter::zero();
        let total = forward_stages.last().unwrap().count_boards();

        crossbeam::scope(|s| {
            s.spawn(|_| loop {
                let counted = counter.get();
                eprint!(
                    "\r{:>12} / {:>12} ({:>6.2}%)",
                    counted,
                    total,
                    (counted as f64) / (total as f64) * 100.
                );
                std::io::stdout().flush().unwrap();
                if counted == total as u64 {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            });

            forward_stages.push(forward_stages.last().unwrap().step(&counter));
        })
        .unwrap();

        eprintln!(
            "  After iteration {}, have {} boards.",
            iter,
            forward_stages.last().unwrap().count_boards()
        );
    }

    let mut all_boards = Vec::new();
    const TARGET_BOARD: Board = Board(0b1111111111_1111111111_1111111111_1111111111);
    all_boards.push(TARGET_BOARD);

    let mut target_stage = forward_stages.pop().unwrap().filter(TARGET_BOARD);

    for (i, stage) in forward_stages.drain(..).enumerate().rev() {
        let counter = Counter::zero();
        let total = stage.0.lock_all().count();

        let this_stage = crossbeam::scope(|s| {
            s.spawn(|_| loop {
                let counted = counter.get();
                eprint!(
                    "\r{:>12} / {:>12} ({:>6.2}%)",
                    counted,
                    total,
                    (counted as f64) / (total as f64) * 100.
                );
                std::io::stdout().flush().unwrap();
                if counted == total as u64 {
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            });

            stage.target(&target_stage, &counter)
        })
        .unwrap();

        eprintln!(
            "  After stage {}, have {} boards.",
            i,
            this_stage.count_boards()
        );

        for (board, _preds) in this_stage.0.lock_all().iter() {
            all_boards.push(board);
        }

        target_stage = this_stage;
    }

    all_boards.par_sort_unstable();

    all_boards
}

pub struct SimpleStage(pub Stage<SmallVec<[Board; 6]>>);

impl SimpleStage {
    pub fn new() -> SimpleStage {
        SimpleStage(Stage::initial(SmallVec::new()))
    }

    pub fn step(&self, counter: &Counter) -> SimpleStage {
        let new_stage = SimpleStage(Stage::empty());

        self.0.lock_all().par_iter().for_each(|(&board, _preds)| {
            Shape::ALL.par_iter().for_each(|&shape| {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    if new_board.has_isolated_cell() || new_board.has_imbalanced_split() {
                        continue;
                    }

                    let mut subset = new_stage.0.lock_subset(new_board);
                    let entry = subset.entry(new_board).or_insert_with(SmallVec::new);

                    if !entry.contains(&board) {
                        entry.push(board);
                    }
                }
            });

            counter.increment();
        });

        new_stage
    }

    pub fn count_boards(&self) -> usize {
        self.0 .0.iter().map(|subset| subset.lock().len()).sum()
    }

    pub fn filter(&self, board: Board) -> SimpleStage {
        let preds = self.0.lock_subset(board).get(&board).unwrap().clone();

        let new_stage = SimpleStage(Stage::empty());
        new_stage.0.lock_subset(board).insert(board, preds);

        new_stage
    }

    pub fn target(&self, target: &SimpleStage, counter: &Counter) -> SimpleStage {
        let target = target.0.lock_all();
        let new_stage = SimpleStage(Stage::empty());

        let target_preds: HashSet<Board> = target
            .iter()
            .flat_map(|(_board, preds)| preds)
            .copied()
            .collect();

        self.0.lock_all().par_iter().for_each(|(&board, preds)| {
            if !target_preds.contains(&board) {
                counter.increment();
                return;
            }

            for &shape in &Shape::ALL {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    if target.get(new_board).is_some() {
                        new_stage.0.lock_subset(board).insert(board, preds.clone());
                        counter.increment();
                        return;
                    }
                }
            }

            counter.increment();
        });

        new_stage
    }
}
