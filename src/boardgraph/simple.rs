use rayon::prelude::*;

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

pub struct SimpleStage(pub Stage<()>);

impl SimpleStage {
    pub fn new() -> SimpleStage {
        SimpleStage(Stage::initial(()))
    }

    pub fn step(&self) -> SimpleStage {
        let new_stage = SimpleStage(Stage::empty());

        self.0
            .lock_all()
            .par_iter()
            .flat_map(|(&board, &())| Shape::ALL.par_iter().map(move |&shape| (board, shape)))
            .for_each(|(board, shape)| {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    let mut subset = new_stage.0.lock_subset(new_board);

                    subset.insert(new_board, ());
                }
            });

        new_stage
    }

    pub fn count_boards(&self) -> usize {
        self.0 .0.iter().map(|subset| subset.lock().len()).sum()
    }

    pub fn filter(&self, board: Board) -> SimpleStage {
        assert!(self.0.lock_subset(board).get(&board).is_some());

        let new_stage = SimpleStage(Stage::empty());
        new_stage.0.lock_subset(board).insert(board, ());

        new_stage
    }

    pub fn target(&self, target: &SimpleStage) -> SimpleStage {
        let target = target.0.lock_all();
        let new_stage = SimpleStage(Stage::empty());

        self.0.lock_all().par_iter().for_each(|(&board, &())| {
            for &shape in &Shape::ALL {
                for (_, new_board) in PiecePlacer::new(board, shape) {
                    if target.get(new_board).is_some() {
                        new_stage.0.lock_subset(board).insert(board, ());
                        return;
                    }
                }
            }
        });

        new_stage
    }
}
