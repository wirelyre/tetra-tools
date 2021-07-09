use bitvec::bitvec;
use rayon::prelude::*;

use super::Stage;
use crate::gameplay::{Piece, Shape};

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
                let piece = Piece::new(shape);
                let mut queue = vec![piece];
                let mut seen = bitvec![0; 0x4000];
                seen.set(piece.pack() as usize, true);

                while let Some(piece) = queue.pop() {
                    for &new_piece in &[
                        piece.left(board),
                        piece.right(board),
                        piece.down(board),
                        piece.cw(board),
                        piece.ccw(board),
                    ] {
                        if !seen[new_piece.pack() as usize] {
                            seen.set(new_piece.pack() as usize, true);

                            queue.push(new_piece);

                            if new_piece.can_place(board) {
                                let new_board = new_piece.place(board);
                                let mut subset = new_stage.0.lock_subset(new_board);

                                subset.insert(new_board, ());
                            }
                        }
                    }
                }
            });

        new_stage
    }

    pub fn count_boards(&self) -> usize {
        self.0 .0.iter().map(|subset| subset.lock().len()).sum()
    }
}
