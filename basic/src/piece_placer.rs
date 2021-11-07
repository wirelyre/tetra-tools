use bitvec::prelude::{bitvec, BitVec};

use crate::gameplay::{Board, Piece, Shape};

pub struct PiecePlacer {
    board: Board,
    queue: Vec<Piece>,
    seen: BitVec,
}

impl PiecePlacer {
    pub fn new(board: Board, shape: Shape) -> PiecePlacer {
        let piece = Piece::new(shape);
        let queue = vec![piece];
        let mut seen = bitvec![0; 0x4000];

        seen.set(piece.pack() as usize, true);

        PiecePlacer { board, queue, seen }
    }
}

impl Iterator for PiecePlacer {
    type Item = (Piece, Board);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let piece = self.queue.pop()?;

            for &new_piece in &[
                piece.left(self.board),
                piece.right(self.board),
                piece.down(self.board),
                piece.cw(self.board),
                piece.ccw(self.board),
            ] {
                if !self.seen[new_piece.pack() as usize] {
                    self.seen.set(new_piece.pack() as usize, true);
                    self.queue.push(new_piece);
                }
            }

            if piece.can_place(self.board) {
                return Some((piece, piece.place(self.board)));
            }
        }
    }
}
