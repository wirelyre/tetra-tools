pub mod gamestate;
pub mod simple;

use std::collections::HashMap;

use bitvec::prelude::{bitvec, BitVec};
use parking_lot::{Mutex, MutexGuard};
use rayon::prelude::*;

use crate::gameplay::{Board, Piece, Shape};

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

pub struct Stage<T>(pub Vec<Mutex<HashMap<Board, T>>>);
pub struct StageRef<'a, T>(Vec<parking_lot::MutexGuard<'a, HashMap<Board, T>>>);

impl<T> Stage<T> {
    pub fn empty() -> Stage<T> {
        let mut subsets = Vec::new();

        for _ in 0..LOW_BITS_MASK + 1 {
            subsets.push(Mutex::new(HashMap::new()));
        }

        Stage(subsets)
    }

    pub fn initial(val: T) -> Stage<T> {
        let stage = Stage::empty();

        let empty_board = Board::empty();
        stage.lock_subset(empty_board).insert(empty_board, val);

        stage
    }

    pub fn lock_subset(&self, board: Board) -> MutexGuard<'_, HashMap<Board, T>> {
        self.0[(board.0 & LOW_BITS_MASK) as usize].lock()
    }

    pub fn lock_all(&self) -> StageRef<'_, T> {
        let guards: Vec<_> = self.0.iter().map(Mutex::lock).collect();
        StageRef(guards)
    }
}

impl<'a, T> StageRef<'a, T> {
    pub fn get(&self, board: Board) -> Option<&T> {
        self.0[(board.0 & LOW_BITS_MASK) as usize].get(&board)
    }

    pub fn get_mut(&mut self, board: Board) -> Option<&mut T> {
        self.0[(board.0 & LOW_BITS_MASK) as usize].get_mut(&board)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Board, &T)> {
        self.0
            .iter()
            .flat_map(|subset| subset.iter())
            .map(|(&board, val)| (board, val))
    }

    pub fn iter_mut<'b: 'a>(&'b mut self) -> impl Iterator<Item = (Board, &'b mut T)> {
        self.0
            .iter_mut()
            .flat_map(|subset| subset.iter_mut())
            .map(|(&board, val)| (board, val))
    }
}

impl<'a, 'b: 'a, T: Sync> ParallelIterator for &'b StageRef<'a, T> {
    type Item = (&'a Board, &'a T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        self.0
            .par_iter()
            .flat_map(|subset| subset.par_iter())
            .drive_unindexed(consumer)
    }
}

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
