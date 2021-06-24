use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;

use crate::gameplay::Board;

pub struct BoardSet(pub Vec<Arc<Mutex<HashSet<Board>>>>);

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

impl BoardSet {
    pub fn new() -> Self {
        let mut v = Vec::new();

        for _ in 0..(LOW_BITS_MASK + 1) {
            v.push(Arc::new(Mutex::new(HashSet::new())))
        }

        BoardSet(v)
    }

    pub fn get(&self, board: Board) -> bool {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = self.0[low_bits].lock();
        subset.contains(&board)
    }

    pub fn insert(&self, board: Board) {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let mut subset = self.0[low_bits].lock();
        subset.insert(board);
    }
}

pub struct Iter<'a>(&'a BoardSet);

impl<'a> ParallelIterator for Iter<'a> {
    type Item = Board;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        let guards: Vec<_> = self.0 .0.iter().map(|subset| subset.lock()).collect();

        let boards = guards
            .par_iter()
            .flat_map(|guard| guard.par_iter().cloned());

        boards.drive_unindexed(consumer)
    }
}

impl<'a> IntoParallelIterator for &'a BoardSet {
    type Iter = Iter<'a>;

    type Item = Board;

    fn into_par_iter(self) -> Self::Iter {
        Iter(self)
    }
}

impl FromParallelIterator<Board> for BoardSet {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = Board>,
    {
        let set = BoardSet::new();
        par_iter.into_par_iter().for_each(|board| set.insert(board));
        set
    }
}
