use bitvec::prelude::{bitvec, BitVec};

use crate::gameplay::{Board, Orientation, Piece, Shape};

pub struct PiecePlacer {
    board: Board,
    queue: Vec<Piece>,
    seen: BitVec,
}

impl PiecePlacer {
    pub fn new(board: Board, shape: Shape) -> PiecePlacer {
        use Orientation::*;

        let mut queue = Vec::new();
        let mut seen = bitvec![0; 0x4000];

        // This initialization looks scary, but it's free.  We would see most of
        // these pieces anyway.
        //
        // Note that every `piece` here is valid on the board.  They spawn at
        // row 4, above the maximum allowed filled cells.
        for orientation in [North, East, South, West] {
            for col in 0..10 {
                let piece = Piece {
                    shape,
                    col,
                    row: 4,
                    orientation,
                };
                if piece.in_bounds() {
                    queue.push(piece);
                    seen.set(piece.pack() as usize, true);
                }
            }
        }

        let piece = Piece::new(shape);
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
