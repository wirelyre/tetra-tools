use rayon::prelude::*;
use std::collections::HashSet;

use crate::gameplay::Board;

pub struct BoardSet(pub Vec<HashSet<Board>>);

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

impl BoardSet {
    pub fn new() -> Self {
        let mut v = Vec::new();

        for _ in 0..(LOW_BITS_MASK + 1) {
            v.push(HashSet::new());
        }

        BoardSet(v)
    }

    pub fn get(&self, board: Board) -> bool {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = &self.0[low_bits];
        subset.contains(&board)
    }

    pub fn insert(&mut self, board: Board) {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = &mut self.0[low_bits];
        subset.insert(board);
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(HashSet::len).sum()
    }
}

pub struct Iter<'a>(&'a BoardSet);

impl<'a> ParallelIterator for Iter<'a> {
    type Item = Board;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        let boards = self
            .0
             .0
            .par_iter()
            .flat_map(|subset| subset.par_iter().cloned());

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
        let mut set = BoardSet::new();

        crossbeam::scope(|s| {
            let (send, recv) = crossbeam::channel::unbounded();

            let set = &mut set;
            s.spawn(move |_| {
                while let Ok(board) = recv.recv() {
                    set.insert(board);
                }
            });

            par_iter
                .into_par_iter()
                .for_each_with(send, |send, board| send.send(board).unwrap());
        })
        .unwrap();

        set
    }
}
