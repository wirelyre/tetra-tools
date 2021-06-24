use rayon::prelude::*;
use std::collections::HashMap;

use crate::gameplay::Board;

pub struct BoardMap<T>(Vec<HashMap<Board, T>>);
pub struct BoardSet(BoardMap<()>);

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

impl BoardSet {
    pub fn new() -> Self {
        BoardSet(BoardMap::new())
    }

    pub fn get(&self, board: Board) -> bool {
        self.0.get(board).is_some()
    }

    pub fn insert(&mut self, board: Board) {
        self.0.insert(board, ())
    }

    pub fn count(&self) -> usize {
        self.0.count()
    }
}

impl<T> BoardMap<T> {
    pub fn new() -> Self {
        let mut v = Vec::new();

        for _ in 0..(LOW_BITS_MASK + 1) {
            v.push(HashMap::new());
        }

        BoardMap(v)
    }

    pub fn get(&self, board: Board) -> Option<&T> {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = &self.0[low_bits];
        subset.get(&board)
    }

    pub fn insert(&mut self, board: Board, value: T) {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = &mut self.0[low_bits];
        subset.insert(board, value);
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(HashMap::len).sum()
    }
}

pub struct SetIter<'a>(&'a BoardSet);
impl<'a> ParallelIterator for SetIter<'a> {
    type Item = Board;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        self.0
             .0
            .par_iter()
            .map(|(board, ())| board)
            .drive_unindexed(consumer)
    }
}
impl<'a> IntoParallelIterator for &'a BoardSet {
    type Iter = SetIter<'a>;

    type Item = Board;

    fn into_par_iter(self) -> Self::Iter {
        SetIter(&self)
    }
}
impl FromParallelIterator<Board> for BoardSet {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = Board>,
    {
        let map: BoardMap<()> = par_iter.into_par_iter().map(|board| (board, ())).collect();
        BoardSet(map)
    }
}

pub struct MapIter<'a, T>(&'a BoardMap<T>);

impl<'a, T: Sync> ParallelIterator for MapIter<'a, T> {
    type Item = (Board, &'a T);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        let boards = self
            .0
             .0
            .par_iter()
            .flat_map(|subset| subset.par_iter().map(|(&board, value)| (board, value)));

        boards.drive_unindexed(consumer)
    }
}

impl<'a, T: Sync> IntoParallelIterator for &'a BoardMap<T> {
    type Iter = MapIter<'a, T>;

    type Item = (Board, &'a T);

    fn into_par_iter(self) -> Self::Iter {
        MapIter(self)
    }
}

impl<T: Send> FromParallelIterator<(Board, T)> for BoardMap<T> {
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = (Board, T)>,
    {
        let mut set = BoardMap::new();

        crossbeam::scope(|s| {
            let (send, recv) = crossbeam::channel::unbounded();

            let set = &mut set;
            s.spawn(move |_| {
                while let Ok((board, value)) = recv.recv() {
                    set.insert(board, value);
                }
            });

            par_iter
                .into_par_iter()
                .for_each_with(send, |send, (board, value)| {
                    send.send((board, value)).unwrap()
                });
        })
        .unwrap();

        set
    }
}
