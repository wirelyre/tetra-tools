use rayon::prelude::*;

use super::Stage;
use crate::{boardgraph::PiecePlacer, gameplay::Shape};

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
}
