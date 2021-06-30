use std::collections::HashMap;

use bitvec::bitvec;
use parking_lot::Mutex;
use rayon::prelude::*;

use crate::gameplay::{Board, Piece, Shape};

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

pub struct GameStateGraph(pub Vec<Mutex<HashMap<Board, QuantumBag>>>);

impl GameStateGraph {
    pub fn empty() -> GameStateGraph {
        let mut subsets = Vec::new();

        for _ in 0..LOW_BITS_MASK + 1 {
            subsets.push(Mutex::new(HashMap::new()));
        }

        GameStateGraph(subsets)
    }

    pub fn new(first_bag: QuantumBag) -> GameStateGraph {
        let me = GameStateGraph::empty();

        let empty_board = Board::empty();
        me.0[(empty_board.0 & LOW_BITS_MASK) as usize]
            .lock()
            .insert(empty_board, first_bag);

        me
    }

    pub fn step(&self) -> GameStateGraph {
        let new_graph = GameStateGraph::empty();
        let guards: Vec<_> = self.0.iter().map(Mutex::lock).collect();

        guards
            .par_iter()
            .flat_map(|subset| subset.par_iter())
            .flat_map(|(&board, quantum_bag)| {
                quantum_bag
                    .par_iter_take_one()
                    .map(move |(shape, updater)| (board, shape, updater))
            })
            .for_each(|(board, shape, updater)| {
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
                                let mut subset =
                                    new_graph.0[(new_board.0 & LOW_BITS_MASK) as usize].lock();

                                let new_quantum_bag =
                                    subset.entry(new_board).or_insert_with(QuantumBag::empty);
                                updater.update(new_quantum_bag);
                            }
                        }
                    }
                }
            });

        new_graph
    }

    pub fn count(&self) -> usize {
        self.0.iter().map(|subset| subset.lock().len()).sum()
    }

    pub fn count_bags(&self) -> usize {
        self.0
            .iter()
            .map(|subset| {
                subset
                    .lock()
                    .iter()
                    .map(|(_, quantum_bag)| quantum_bag.0.len())
                    .sum::<usize>()
            })
            .sum()
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bag(u8);

impl Bag {
    pub fn full() -> Bag {
        Bag(0b1111111)
    }

    pub fn has(self, shape: Shape) -> bool {
        (self.0 & shape.bit_mask()) != 0
    }

    pub fn without(self, shape: Shape) -> Bag {
        let bits = self.0 & !shape.bit_mask();

        if bits == 0 {
            Bag::full()
        } else {
            Bag(bits)
        }
    }

    fn either(self, other: Bag) -> Bag {
        Bag(self.0 | other.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantumBag(Vec<Bag>);

impl QuantumBag {
    pub fn new(initial: Bag) -> QuantumBag {
        QuantumBag(vec![initial])
    }

    pub fn empty() -> QuantumBag {
        QuantumBag(Vec::new())
    }

    pub fn par_iter_take_one(&self) -> QuantumBagTakeOneParIter<'_> {
        let available_pieces = self.0.iter().cloned().fold(Bag(0), Bag::either);

        QuantumBagTakeOneParIter {
            available_pieces,
            slice: &self.0,
        }
    }
}

pub struct QuantumBagTakeOneParIter<'a> {
    available_pieces: Bag,
    slice: &'a [Bag],
}

impl<'a> ParallelIterator for QuantumBagTakeOneParIter<'a> {
    type Item = (Shape, QuantumBagUpdater<'a>);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        let all_shapes: [Shape; 7] = [
            Shape::I,
            Shape::J,
            Shape::L,
            Shape::O,
            Shape::S,
            Shape::T,
            Shape::Z,
        ];

        all_shapes
            .into_par_iter()
            .filter(|shape| self.available_pieces.has(*shape))
            .map(|shape| {
                (
                    shape,
                    QuantumBagUpdater {
                        shape,
                        old: self.slice,
                    },
                )
            })
            .drive_unindexed(consumer)
    }
}

pub struct QuantumBagUpdater<'a> {
    shape: Shape,
    old: &'a [Bag],
}

impl<'a> QuantumBagUpdater<'a> {
    pub fn update(&self, quantum_bag: &mut QuantumBag) {
        for old_bag in self.old {
            if old_bag.has(self.shape) {
                let new_bag = old_bag.without(self.shape);

                if !quantum_bag.0.contains(&new_bag) {
                    quantum_bag.0.push(new_bag);
                }
            }
        }
    }
}
