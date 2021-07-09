use std::collections::HashSet;

use rayon::prelude::*;
use smallvec::SmallVec;

use super::Stage;
use crate::{
    boardgraph::PiecePlacer,
    gameplay::{Board, Shape},
};

pub struct SimpleGraph(pub Vec<SimpleStage>);

impl SimpleGraph {
    pub fn compute() -> SimpleGraph {
        let mut forward_stages = Vec::new();
        forward_stages.push(SimpleStage::new());

        for iter in 1..=4 {
            forward_stages.push(forward_stages.last().unwrap().step());
            println!(
                "After iteration {}, have {} boards.",
                iter,
                forward_stages.last().unwrap().count_boards()
            );
        }

        let mut backward_stages = Vec::new();
        let mut target_stage = forward_stages
            .pop()
            .unwrap()
            .filter(Board(0b0000000111_0000001111_0000011111_0000001111));

        while let Some(stage) = forward_stages.pop() {
            let this_stage = stage.target(&target_stage);
            backward_stages.push(target_stage);
            target_stage = this_stage;
        }

        backward_stages.push(target_stage);
        backward_stages.reverse();

        for (i, stage) in backward_stages.iter().enumerate() {
            println!("After stage {}, have {} boards.", i, stage.count_boards());
        }

        SimpleGraph(backward_stages)
    }
}

pub struct SimpleStage(pub Stage<SmallVec<[Board; 6]>>);

impl SimpleStage {
    pub fn new() -> SimpleStage {
        SimpleStage(Stage::initial(SmallVec::new()))
    }

    pub fn step(&self) -> SimpleStage {
        let new_stage = SimpleStage(Stage::empty());

        self.0
            .lock_all()
            .par_iter()
            .flat_map(|(&board, _preds)| Shape::ALL.par_iter().map(move |&shape| (board, shape)))
            .for_each(|(board, shape)| {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    let mut subset = new_stage.0.lock_subset(new_board);
                    let entry = subset.entry(new_board).or_insert_with(SmallVec::new);

                    if !entry.contains(&board) {
                        entry.push(board);
                    }
                }
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

    pub fn target(&self, target: &SimpleStage) -> SimpleStage {
        let target = target.0.lock_all();
        let new_stage = SimpleStage(Stage::empty());

        let target_preds: HashSet<Board> = target
            .iter()
            .flat_map(|(_board, preds)| preds)
            .copied()
            .collect();

        self.0.lock_all().par_iter().for_each(|(&board, preds)| {
            if !target_preds.contains(&board) {
                return;
            }

            for &shape in &Shape::ALL {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    if target.get(new_board).is_some() {
                        new_stage.0.lock_subset(board).insert(board, preds.clone());
                        return;
                    }
                }
            }
        });

        new_stage
    }
}
