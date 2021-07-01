use std::collections::HashMap;

use bitvec::bitvec;
use parking_lot::Mutex;
use rayon::prelude::*;
use smallvec::{smallvec, SmallVec};

use crate::gameplay::{Board, Piece, Shape};

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

pub struct GameStateGraph(pub Vec<Mutex<HashMap<Board, QuantumBag>>>);
pub struct GraphRef<'a>(Vec<parking_lot::MutexGuard<'a, HashMap<Board, QuantumBag>>>);

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

impl<'a> GraphRef<'a> {
    pub fn get(&self, board: Board) -> Option<&QuantumBag> {
        self.0[(board.0 & LOW_BITS_MASK) as usize].get(&board)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum MaybeShape {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
    None,
}

impl From<Shape> for MaybeShape {
    fn from(shape: Shape) -> Self {
        match shape {
            Shape::I => MaybeShape::I,
            Shape::J => MaybeShape::J,
            Shape::L => MaybeShape::L,
            Shape::O => MaybeShape::O,
            Shape::S => MaybeShape::S,
            Shape::T => MaybeShape::T,
            Shape::Z => MaybeShape::Z,
        }
    }
}

impl From<Option<Shape>> for MaybeShape {
    fn from(option_shape: Option<Shape>) -> Self {
        match option_shape {
            Some(shape) => shape.into(),
            None => MaybeShape::None,
        }
    }
}

impl From<MaybeShape> for Option<Shape> {
    fn from(maybe_shape: MaybeShape) -> Self {
        match maybe_shape {
            MaybeShape::I => Some(Shape::I),
            MaybeShape::J => Some(Shape::J),
            MaybeShape::L => Some(Shape::L),
            MaybeShape::O => Some(Shape::O),
            MaybeShape::S => Some(Shape::S),
            MaybeShape::T => Some(Shape::T),
            MaybeShape::Z => Some(Shape::Z),
            MaybeShape::None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bag {
    shapes: u8,
    hold: MaybeShape,
}

impl Bag {
    pub fn full_no_hold() -> Bag {
        Bag {
            shapes: 0b1111111,
            hold: None.into(),
        }
    }

    pub fn take(self, shape: Shape) -> QuantumBag {
        let mut result = QuantumBag::empty();

        if self.has(shape) {
            result.0.push(self.without(shape));
        }

        if self.hold == shape.into() {
            result.0.push(Bag {
                shapes: self.shapes,
                hold: None.into(),
            });
        } else if self.hold == None.into() {
            for &hold_shape in &Shape::ALL {
                if self.has(hold_shape) {
                    let mut new = self.without(hold_shape);
                    new.hold = hold_shape.into();

                    if new.has(shape) {
                        result.0.push(new.without(shape));
                    }
                }
            }
        }

        result
    }

    fn has(self, shape: Shape) -> bool {
        (self.shapes & shape.bit_mask()) != 0
    }

    fn without(self, shape: Shape) -> Bag {
        let shapes = self.shapes & !shape.bit_mask();
        let shapes = if shapes == 0 { 0b1111111 } else { shapes };

        Bag {
            shapes,
            hold: self.hold,
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuantumBag(SmallVec<[Bag; 8]>);

impl QuantumBag {
    pub fn new(initial: Bag) -> QuantumBag {
        QuantumBag(smallvec![initial])
    }

    pub fn empty() -> QuantumBag {
        QuantumBag(SmallVec::new())
    }

    pub fn every_bag_no_hold() -> QuantumBag {
        let each_bits = (0b0000001..=0b1111111).into_iter();

        QuantumBag(
            each_bits
                .map(|bits| Bag {
                    shapes: bits,
                    hold: None.into(),
                })
                .collect(),
        )
    }

    pub fn available_pieces(&self) -> u8 {
        let mut result = 0;

        for &bag in &self.0 {
            result |= bag.shapes;

            let shape: Option<Shape> = bag.hold.into();
            if let Some(shape) = shape {
                result |= shape.bit_mask();
            }
        }

        result
    }

    pub fn par_iter_take_one(&self) -> QuantumBagTakeOneParIter<'_> {
        QuantumBagTakeOneParIter {
            available_pieces: self.available_pieces(),
            slice: &self.0,
        }
    }
}

pub struct QuantumBagTakeOneParIter<'a> {
    available_pieces: u8,
    slice: &'a [Bag],
}

impl<'a> ParallelIterator for QuantumBagTakeOneParIter<'a> {
    type Item = (Shape, QuantumBagUpdater<'a>);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        Shape::ALL
            .into_par_iter()
            .filter(|shape| (self.available_pieces & shape.bit_mask()) != 0)
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
            for new_bag in old_bag.take(self.shape).0 {
                if !quantum_bag.0.contains(&new_bag) {
                    quantum_bag.0.push(new_bag);
                }
            }
        }
    }
}

impl std::fmt::Display for Bag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for &shape in &Shape::ALL {
            if self.has(shape) {
                f.write_str(shape.name())?;
            }
        }

        let hold: Option<Shape> = self.hold.into();
        if let Some(shape) = hold {
            write!(f, " ({})", shape.name())?;
        }

        Ok(())
    }
}

impl std::fmt::Display for QuantumBag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bags: Vec<_> = self.0.iter().collect();
        bags.sort_unstable();

        write!(f, "QuantumBag:\n")?;

        for bag in bags {
            write!(f, "    {}\n", bag)?;
        }

        Ok(())
    }
}
