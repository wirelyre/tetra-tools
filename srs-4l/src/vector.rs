//! Vector implementation of 4-line [SRS](https://harddrop.com/wiki/SRS) and two
//! half-rotation extensions.  Computes all piece placements in parallel.
//!
//! The core item is [`Placements`], which performs the search when
//! constructed, and then acts as an iterator over possible pieces.
//!
//! ## How
//!
//! (See also [`Piece`].)
//!
//! For a given [`Board`], [`Shape`], and [`Orientation`], there are two
//! interesting sets of positions:
//!
//! 1. **Viable** positions: Positions where a piece could be placed if it were
//!    teleported there.
//! 2. **Reachable** positions: Positions where a piece can be placed by a
//!    sequence of movements.
//!
//! The first is easy.  A position is **viable** if, when a piece is placed
//! there, none of the minoes would overlap with filled cells.  Viable positions
//! can be computed quickly from a board, shape, and orientation.  This is
//! handled by [`Collision`].
//!
//! The second is trickier.  A position is **reachable** if there is a path to
//! it from other reachable positions.  So there must be an initial set of
//! [`SPAWN`] positions from which other positions can be reached.  Reachable
//! positions lead to other reachable positions, but the specific paths might be
//! complex.
//!
//! This module uses position vectors, [`PVec`], which represent many positions
//! simultaneously.  Each piece movement is applied to every reachable position
//! in the vector to produce a new set of reachable positions.  The new set
//! contains the old set --- a reachable position vector represents *all known
//! reachable positions*, not just newly discovered positions.
//!
//! For an overview of the algorithm:
//!
//! 1. All *viable* positions are found for each orientation.
//! 2. The *reachable* positions are initialized with *spawn* positions.
//! 3. Positions are moved left, right, down, clockwise, and counter-clockwise
//!    according to SRS rules for the given piece and each orientation.
//! 4. Step 3 is repeated until no new *reachable* positions are discovered.
//! 5. All *placeable* positions are found: positions that are *reachable*, in
//!    bounds, and sit on something.
//!
//! Then the placeable positions in the resulting vectors can be used one by
//! one as desired.
//!
//! The details are delegated to [`Placements`], [`PlacementMachine`],
//! [`Collision`], and [`Kicks`].  `Collision` and `Kicks` do some work at
//! compile time to simplify the work necessary at runtime.
//!
//! ## Why?
//!
//! It's **very, very fast**.
//!
//! Storing position data separately from a board is straightforward, but
//! using the same bit representation as the board is much thriftier.  It turns
//! out that SRS movements can be done directly on the bit representation.  The
//! operations are tricky but simple.
//!
//! Also, **many positions are considered simultaneously**.  A single movement
//! might discover 10 or more new reachable positions.
//!
//! Finally, it's just as easy to **find all placeable positions** as to find a
//! single one.  The resulting [`Placements`] structure can be used either as an
//! iterator or as a set with query operations.  It is never necessary to
//! iterate through all placements to try to find a single specific one.
//!
//! ## Why not?
//!
//! This method loses information about the path a piece takes to reach a
//! position.  In order to actually play a game, you would have to know which
//! movements to perform in order to place a piece somewhere.  It is also often
//! useful to prioritize quick placements over slow ones, but there is no way to
//! keep track using this method --- all possible positions are considered
//! equal.
//!
//! Even though this method is possible to adapt for boards larger than 4 lines,
//! doing so is even trickier than writing this module.  And this module was
//! pretty tricky already.  It's very difficult to check whether code like this
//! is working the way you expect.
//!
//! ## Does it work?
//!
//! It seems so.  This code and the code in [`gameplay`] and [`piece_placer`]
//! produce exactly the same results on the boards resulting from
//! `precompute`.  In particular, both methods result in a perfectly identical
//! precomputed file --- but this method is more than 10 times faster.  :-)
//!
//! [`gameplay`]:     crate::gameplay
//! [`piece_placer`]: crate::piece_placer

use crate::gameplay::{Board, Orientation, Physics, Piece, Shape};

/// Vector of positions on a board.
///
/// Whereas a [`Board`] represents a single actual board, with set bits for
/// filled *cells*, a `PVec` represents multiple positions on a board, possibly
/// of objects which have multiple minoes, with set bits for *positions* under
/// consideration.
///
/// Since this is a set of positions, there are implementations of a few bitwise
/// operators.  Any other uses should use the wrapped bits directly.
///
/// This should always have bits 60–64 unset.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PVec(pub u64);

/// Searcher, iterator, and queryable set for placeable positions of a given
/// shape on a given board.
///
/// `Placements` is [`Iterator`], [`DoubleEndedIterator`], and
/// [`ExactSizeIterator`].  It represents a single set of pieces with
/// orientations and positions, and this set has a well-defined and stable order
/// (ordered first by orientation, then by position).
///
/// It is also [`Eq`], [`Hash`], and [`Ord`].  If two `Placements` objects are
/// not equal, then they represent truly different sets of pieces.
///
/// Note that using `Placements` as an iterator will drain pieces out of it.
/// It's pretty cheap to `clone`, so do that if you want to keep the original
/// set.
///
/// The number of pieces in the set is given by [`len`](Placements::len).
#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Placements {
    /// Shape of the pieces placed.
    pub shape: Shape,
    /// Original board which pieces are placed onto.
    pub board: Board,
    /// Placeable positions, indexed by orientation.
    pub positions: [PVec; 4],
}

impl Placements {
    /// Find all placeable positions of the given shape on the given board.
    ///
    /// This method keeps an internal state, [`PlacementMachine`], updating
    /// reachable positions until they stop changing.  Then it finds all
    /// placeable positions and returns them.
    ///
    /// See [`PlacementMachine`] for details.
    pub fn place(board: Board, shape: Shape, physics: Physics) -> Self {
        use Orientation::*;

        let collision = &COLLISION[shape as usize];

        if shape == Shape::O {
            // Shortcut for O.
            // - An O piece can never move upwards (no up-kicks)
            // - All O orientations are completely identical

            let viable = collision[0].viable(board);
            let reachable = (SPAWN & viable).flood_fill(viable);
            let placeable = collision[0].placeable(reachable);

            return Placements {
                shape,
                board,
                positions: [placeable; 4],
            };
        }

        let viable = [
            collision[0].viable(board),
            collision[1].viable(board),
            collision[2].viable(board),
            collision[3].viable(board),
        ];
        let reachable = [
            SPAWN & viable[0],
            SPAWN & viable[1],
            SPAWN & viable[2],
            SPAWN & viable[3],
        ];
        let mut machine = PlacementMachine {
            viable,
            reachable,
            dirty: [true; 4],
            shape,
            physics,
        };

        while machine.any_dirty() {
            machine.step(North);
            machine.step(East);
            machine.step(South);
            machine.step(West);
        }

        Placements {
            shape,
            board,
            positions: [
                machine.placeable(North),
                machine.placeable(East),
                machine.placeable(South),
                machine.placeable(West),
            ],
        }
    }

    /// Combine orientations that look the same.
    ///
    /// For example, with the S piece, the north and south orientations look the
    /// same, even though they rotate differently.
    pub fn canonical(self) -> Self {
        use Shape::*;

        match self.shape {
            O => Placements {
                // 90° symmetry, all orientations identical
                positions: [self.positions[0], PVec(0), PVec(0), PVec(0)],
                ..self
            },

            I | S | Z => Placements {
                // 180° symmetry
                positions: [
                    self.positions[0] | self.positions[2],
                    self.positions[1] | self.positions[3],
                    PVec(0),
                    PVec(0),
                ],
                ..self
            },

            // not symmetrical
            J | L | T => self,
        }
    }

    /// Check whether the given piece is in this set of placements.
    pub fn contains(&self, piece: Piece) -> bool {
        self.shape == piece.shape
            && self.positions[piece.orientation as usize].contains(piece.col, piece.row)
    }

    /// Remove the given piece from this set of placements.  Returns true if the
    /// piece was initially present, or false if it wasn't.
    pub fn remove(&mut self, piece: Piece) -> bool {
        self.shape == piece.shape
            && self.positions[piece.orientation as usize].remove(piece.col, piece.row)
    }
}

/// The core of the vectorized algorithm.  Not intended for public use.
pub struct PlacementMachine {
    /// Shape of the pieces being placed.  **Constant** during iteration.
    shape: Shape,
    /// Physics for half rotations.  **Constant** during iteration.
    physics: Physics,
    /// Set of viable positions, indexed by orientation.  **Constant** during iteration.
    viable: [PVec; 4],
    /// Set of reachable positions, indexed by orientation.  **Variable** during iteration.
    reachable: [PVec; 4],
    /// Which `reachable` sets need to be visited.  **Variable** during iteration.
    dirty: [bool; 4],
}

impl PlacementMachine {
    /// Check whether any reachable sets need to be visited.  If false,
    /// iteration is complete.
    fn any_dirty(&self) -> bool {
        self.dirty.iter().any(|b| *b)
    }

    /// Visit a single orientation.  If dirty, [flood fills] the reachable
    /// positions, then computes [kicks] in all directions.  If new reachable
    /// positions are discovered during kicks, those other orientations are
    /// marked dirty.
    ///
    /// [flood fills]: PVec::flood_fill
    /// [kicks]:       Kicks
    fn step(&mut self, o: Orientation) {
        let o_0 = o as usize;
        let o_90 = o.cw() as usize;
        let o_180 = o.half() as usize;
        let o_270 = o.ccw() as usize;

        if self.dirty[o_0] {
            self.reachable[o_0] = self.reachable[o_0].flood_fill(self.viable[o_0]);

            let (more_90, more_180, more_270) = match (self.physics, self.shape) {
                // O pieces are handled in the shortcut in `Placements::place`.
                /* (_, Shape::O) => (PVec(0), PVec(0), PVec(0)), */
                (_, Shape::O) => unreachable!(),

                (Physics::SRS, Shape::I) => (
                    SRS_I.cw(o, self.reachable[o_0], self.viable[o_90]),
                    SRS_I.half(o, self.reachable[o_0], self.viable[o_180]),
                    SRS_I.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),
                (Physics::SRS, _) => (
                    SRS_JLSTZ.cw(o, self.reachable[o_0], self.viable[o_90]),
                    SRS_JLSTZ.half(o, self.reachable[o_0], self.viable[o_180]),
                    SRS_JLSTZ.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),

                (Physics::Jstris, Shape::I) => (
                    JSTRIS_I.cw(o, self.reachable[o_0], self.viable[o_90]),
                    JSTRIS_I.half(o, self.reachable[o_0], self.viable[o_180]),
                    JSTRIS_I.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),
                (Physics::Jstris, _) => (
                    JSTRIS_JLSTZ.cw(o, self.reachable[o_0], self.viable[o_90]),
                    JSTRIS_JLSTZ.half(o, self.reachable[o_0], self.viable[o_180]),
                    JSTRIS_JLSTZ.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),

                (Physics::Tetrio, Shape::I) => (
                    TETRIO_I.cw(o, self.reachable[o_0], self.viable[o_90]),
                    TETRIO_I.half(o, self.reachable[o_0], self.viable[o_180]),
                    TETRIO_I.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),
                (Physics::Tetrio, _) => (
                    TETRIO_JLSTZ.cw(o, self.reachable[o_0], self.viable[o_90]),
                    TETRIO_JLSTZ.half(o, self.reachable[o_0], self.viable[o_180]),
                    TETRIO_JLSTZ.ccw(o, self.reachable[o_0], self.viable[o_270]),
                ),
            };

            if (self.reachable[o_90] & more_90) != more_90 {
                self.reachable[o_90] |= more_90;
                self.dirty[o_90] = true;
            }

            if (self.reachable[o_180] & more_180) != more_180 {
                self.reachable[o_180] |= more_180;
                self.dirty[o_180] = true;
            }

            if (self.reachable[o_270] & more_270) != more_270 {
                self.reachable[o_270] |= more_270;
                self.dirty[o_270] = true;
            }

            self.dirty[o_0] = false;
        }
    }

    /// After iteration, finds [placeable] positions from reachable positions.
    ///
    /// [placeable]: Collision::placeable
    fn placeable(&self, o: Orientation) -> PVec {
        COLLISION[self.shape as usize][o as usize].placeable(self.reachable[o as usize])
    }
}

impl Iterator for Placements {
    type Item = (Piece, Board);

    /// Iterate through orientations clockwise starting from north, least
    /// significant bit (lowest mino) first.
    fn next(&mut self) -> Option<Self::Item> {
        use Orientation::*;

        for orientation in [North, East, South, West] {
            let positions = &mut self.positions[orientation as usize];

            if positions.0 != 0 {
                let cell = positions.0.trailing_zeros() as i8;
                let col = cell % 10;
                let row = cell / 10;

                positions.0 ^= 1 << cell;

                let piece = Piece {
                    shape: self.shape,
                    col,
                    row,
                    orientation,
                };

                return Some((piece, piece.place(self.board)));
            }
        }

        None
    }

    /// Count the number of positions in this set.  This is fast.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.positions.iter().copied().map(PVec::count).sum();
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Placements {
    /// Iterate through orientations counter-clockwise starting from west, most
    /// significant bit (highest mino) first.
    fn next_back(&mut self) -> Option<Self::Item> {
        use Orientation::*;

        for orientation in [West, South, East, North] {
            let positions = &mut self.positions[orientation as usize];

            if positions.0 != 0 {
                let cell = 63 - positions.0.leading_zeros() as i8;
                let col = cell / 10;
                let row = cell % 10;

                positions.0 ^= 1 << cell;

                let piece = Piece {
                    shape: self.shape,
                    col,
                    row,
                    orientation,
                };

                return Some((piece, piece.place(self.board)));
            }
        }

        None
    }
}

impl ExactSizeIterator for Placements {}

// Some useful constants.

/// One row: the lowest 10 bits set.
pub const FULL_10: u64 = 0x3FF;
/// Six rows: the lowest 60 bits set.
pub const FULL_60: u64 = 0xFFFFFFFFFFFFFFF;

/// All positions in a 6-high board, except the rightmost column.
pub const LEFT_50: PVec = PVec(replicate_row(0b0111111111));
/// All positions in a 6-high board, except the leftmost column.
pub const RIGHT_50: PVec = PVec(replicate_row(0b1111111110));

/// Spawn positions: all of the fifth and sixth rows.
pub const SPAWN: PVec = PVec((FULL_10 << 50) | (FULL_10 << 40));

impl PVec {
    /// Number of positions in this vector.
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    /// All reachable positions in this vector, plus all positions reachable by
    /// moving down once.
    #[must_use]
    pub fn or_down(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 >> 10 & viable.0))
    }

    /// All reachable positions in this vector, plus all positions reachable by
    /// moving left once.
    #[must_use]
    pub fn or_left(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 >> 1 & LEFT_50.0 & viable.0))
    }

    /// All reachable positions in this vector, plus all positions reachable by
    /// moving right once.
    #[must_use]
    pub fn or_right(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 << 1 & RIGHT_50.0 & viable.0))
    }

    /// All positions reachable from this vector by *any number* of movements
    /// down, left, or right.
    #[must_use]
    pub fn flood_fill(mut self, viable: PVec) -> PVec {
        let mut next;
        while {
            next = self.or_down(viable);
            next = next.or_left(viable);
            next = next.or_right(viable);
            self != next
        } {
            self = next;
        }
        self
    }

    /// Check whether the provided position is in this set.
    pub const fn contains(self, col: i8, row: i8) -> bool {
        self.0 & (1 << (col + row * 10)) != 0
    }

    /// Remove the provided position from this set.  Returns true if the
    /// position was initally present, or false if it wasn't.
    pub fn remove(&mut self, col: i8, row: i8) -> bool {
        let mask = 1 << (col + row * 10);

        if self.0 & mask == 0 {
            return false;
        }

        self.0 ^= mask;
        true
    }
}

/// Collision data for one piece shape, in one orientation.
///
/// Used to determine viable positions and to remove unplaceable positions from
/// the top of the board.
///
/// For viable positions, a bitboard will be shifted four times: once per mino.
/// These shift each mino into the coordinates of the piece.  If a piece were on
/// the board in a given position, that position would be set in each of these
/// shifted vectors.
///
/// ```text
/// Starting piece: Z facing north
///   ░░░░░░░░░░             ░░░░░░░░░░
///   ░██░░░░░░░ at position ░▒▒░░░░░░░
///   ░░██░░░░░░             ░█▒▒░░░░░░
///   ░░░░░░░░░░             ░░░░░░░░░░
///
///     Shift 0: (-1, 0)             Shift 1: (-2, 0)
/// ░░░░░░░░░░    ░░░░░░░░░░     ░░░░░░░░░░    ░░░░░░░░░░
/// ░▒▒░░░░░░░ => ▒▒░░░░░░░░     ░▒▒░░░░░░░ => ▒░░░░░░░░░
/// ░░█▒░░░░░░    ░█▒░░░░░░░     ░░▒█░░░░░░    ▒█░░░░░░░▒ <- wraps
/// ░░░░░░░░░░    ░░░░░░░░░░     ░░░░░░░░░░    ░░░░░░░░░░
///
///     Shift 2: (0, -1)             Shift 3: (-1, -1)
/// ░░░░░░░░░░    ░░░░░░░░░░     ░░░░░░░░░░    ░░░░░░░░░░
/// ░█▒░░░░░░░ => ░░░░░░░░░░     ░▒█░░░░░░░ => ░░░░░░░░░░
/// ░░▒▒░░░░░░    ▒█░░░░░░░░     ░░▒▒░░░░░░    ▒█░░░░░░░░
/// ░░░░░░░░░░    ░▒▒░░░░░░░     ░░░░░░░░░░    ░▒▒░░░░░░░
/// ```
///
/// Inverting the logic, if a cell is full at *any* of these positions, then the
/// piece *cannot* be placed at that position, because it would collide with one
/// of the full cells.
///
/// We're interested in viable positions, however, so after the `shifts` and
/// combining the vectors, we take the inverse.
///
/// Now there is a problem.  Cells from the board have wrapped around to the
/// other side, so the right part of the vector is garbage.  Fortunately, all of
/// the garbage positions share a property: if the piece were placed there, it
/// would collide with the right side of the board!  We simply `mask` out these
/// positions.
///
/// This is enough for viable positions.  But for placeable positions, a piece
/// placed at the position must also not peek out the top of the board.  In
/// other words, there is a maximum placeable position.  We shift the positions
/// vector left and then right by the same amount, which clears bits above
/// `placeable_shift`.
pub struct Collision {
    shifts: [u8; 4],
    mask: u64,
    placeable_shift: u8,
}

/// Packed kick data for a single piece under a single rotation system.
///
/// Takes the number of offsets for quarter-rotation kicks and half-rotation
/// kicks as type parameters.  Assumes an equal number of clockwise and
/// counter-clockwise kicks.
///
/// Each of `rotates` and `masks` is indexed first (as a tuple) by rotation
/// direction, as in the arguments to [`make`], then by initial [orientation],
/// and finally by kick number.
///
/// [`make`]:      Kicks::make
/// [orientation]: Orientation
pub struct Kicks<const QUARTER: usize, const HALF: usize> {
    // TODO: could have better data locality breaking into individual rotations
    rotates: ([[u8; QUARTER]; 4], [[u8; HALF]; 4], [[u8; QUARTER]; 4]),
    masks: ([[u64; QUARTER]; 4], [[u64; HALF]; 4], [[u64; QUARTER]; 4]),
}

/// Collision data for every tetromino.
///
/// Indexed first by piece [shape](Shape), then by [orientation](Orientation).
pub static COLLISION: [[Collision; 4]; 7] = [
    [
        /* I */
        Collision::make([(0, 0), (1, 0), (2, 0), (3, 0)]),
        Collision::make([(0, 0), (0, 1), (0, 2), (0, 3)]),
        Collision::make([(0, 0), (1, 0), (2, 0), (3, 0)]),
        Collision::make([(0, 0), (0, 1), (0, 2), (0, 3)]),
    ],
    [
        /* J */
        Collision::make([(0, 0), (1, 0), (2, 0), (0, 1)]),
        Collision::make([(0, 0), (0, 1), (0, 2), (1, 2)]),
        Collision::make([(2, 0), (0, 1), (1, 1), (2, 1)]),
        Collision::make([(0, 0), (1, 0), (1, 1), (1, 2)]),
    ],
    [
        /* L */
        Collision::make([(0, 0), (1, 0), (2, 0), (2, 1)]),
        Collision::make([(0, 0), (1, 0), (0, 1), (0, 2)]),
        Collision::make([(0, 0), (0, 1), (1, 1), (2, 1)]),
        Collision::make([(1, 0), (1, 1), (0, 2), (1, 2)]),
    ],
    [
        /* O */
        Collision::make([(0, 0), (1, 0), (0, 1), (1, 1)]),
        Collision::make([(0, 0), (1, 0), (0, 1), (1, 1)]),
        Collision::make([(0, 0), (1, 0), (0, 1), (1, 1)]),
        Collision::make([(0, 0), (1, 0), (0, 1), (1, 1)]),
    ],
    [
        /* S */
        Collision::make([(0, 0), (1, 0), (1, 1), (2, 1)]),
        Collision::make([(1, 0), (0, 1), (1, 1), (0, 2)]),
        Collision::make([(0, 0), (1, 0), (1, 1), (2, 1)]),
        Collision::make([(1, 0), (0, 1), (1, 1), (0, 2)]),
    ],
    [
        /* T */
        Collision::make([(0, 0), (1, 0), (2, 0), (1, 1)]),
        Collision::make([(0, 0), (0, 1), (1, 1), (0, 2)]),
        Collision::make([(1, 0), (0, 1), (1, 1), (2, 1)]),
        Collision::make([(1, 0), (0, 1), (1, 1), (1, 2)]),
    ],
    [
        /* Z */
        Collision::make([(1, 0), (2, 0), (0, 1), (1, 1)]),
        Collision::make([(0, 0), (0, 1), (1, 1), (1, 2)]),
        Collision::make([(1, 0), (2, 0), (0, 1), (1, 1)]),
        Collision::make([(0, 0), (0, 1), (1, 1), (1, 2)]),
    ],
];

/// Kick data for I pieces under SRS.
#[rustfmt::skip]
pub static SRS_I: Kicks<5, 0> = Kicks::make(
    [
        [( 2, -2), ( 0, -2), ( 3, -2), ( 0, -3), ( 3,  0)],
        [(-2,  1), (-3,  1), ( 0,  1), (-3,  3), ( 0,  0)],
        [( 1, -1), ( 3, -1), ( 0, -1), ( 3,  0), ( 0, -3)],
        [(-1,  2), ( 0,  2), (-3,  2), ( 0,  0), (-3,  3)],
    ],
    [[], [], [], []],
    [
        [( 1, -2), ( 0, -2), ( 3, -2), ( 0,  0), ( 3, -3)],
        [(-2,  2), ( 0,  2), (-3,  2), ( 0,  3), (-3,  0)],
        [( 2, -1), ( 3, -1), ( 0, -1), ( 3, -3), ( 0,  0)],
        [(-1,  1), (-3,  1), ( 0,  1), (-3,  0), ( 0,  3)],
    ],
);
/// Kick data for J, L, S, T, and Z pieces under SRS.
#[rustfmt::skip]
pub static SRS_JLSTZ: Kicks<5, 0> = Kicks::make(
    [
        [( 1, -1), ( 0, -1), ( 0,  0), ( 1, -3), ( 0, -3)],
        [(-1,  0), ( 0,  0), ( 0, -1), (-1,  2), ( 0,  2)],
        [( 0,  0), ( 1,  0), ( 1,  1), ( 0, -2), ( 1, -2)],
        [( 0,  1), (-1,  1), (-1,  0), ( 0,  3), (-1,  3)],
    ],
    [[], [], [], []],
    [
        [( 0, -1), ( 1, -1), ( 1,  0), ( 0, -3), ( 1, -3)],
        [(-1,  1), ( 0,  1), ( 0,  0), (-1,  3), ( 0,  3)],
        [( 1,  0), ( 0,  0), ( 0,  1), ( 1, -2), ( 0, -2)],
        [( 0,  0), (-1,  0), (-1, -1), ( 0,  2), (-1,  2)],
    ],
);
/// Kick data for O pieces under SRS, Jstris, and TETRIO.  Look at the type,
/// `Kicks<0, 0>` --- in a sense, O actually has no rotations at all.
pub static SRS_O: Kicks<0, 0> = Kicks::make([[], [], [], []], [[], [], [], []], [[], [], [], []]);

/// Kick data for I pieces under Jstris.  Identical to [`SRS_I`] but with two
/// kick offsets for each half rotation.
///
/// [`SRS_I`]: SRS_I
#[rustfmt::skip]
pub static JSTRIS_I: Kicks<5, 2> = Kicks::make(
    [
        [( 2, -2), ( 0, -2), ( 3, -2), ( 0, -3), ( 3,  0)],
        [(-2,  1), (-3,  1), ( 0,  1), (-3,  3), ( 0,  0)],
        [( 1, -1), ( 3, -1), ( 0, -1), ( 3,  0), ( 0, -3)],
        [(-1,  2), ( 0,  2), (-3,  2), ( 0,  0), (-3,  3)],
    ],
    [
        [( 0, -1), ( 0,  0)],
        [(-1,  0), ( 0,  0)],
        [( 0,  1), ( 0,  0)],
        [( 1,  0), ( 0,  0)],
    ],
    [
        [( 1, -2), ( 0, -2), ( 3, -2), ( 0,  0), ( 3, -3)],
        [(-2,  2), ( 0,  2), (-3,  2), ( 0,  3), (-3,  0)],
        [( 2, -1), ( 3, -1), ( 0, -1), ( 3, -3), ( 0,  0)],
        [(-1,  1), (-3,  1), ( 0,  1), (-3,  0), ( 0,  3)],
    ],
);
/// Kick data for J, L, S, T, and Z pieces under Jstris.  Identical to
/// [`SRS_JLSTZ`] but with two kick offsets for each half rotation.
///
/// [`SRS_JLSTZ`]: SRS_JLSTZ
#[rustfmt::skip]
pub static JSTRIS_JLSTZ: Kicks<5, 2> = Kicks::make(
    [
        [( 1, -1), ( 0, -1), ( 0,  0), ( 1, -3), ( 0, -3)],
        [(-1,  0), ( 0,  0), ( 0, -1), (-1,  2), ( 0,  2)],
        [( 0,  0), ( 1,  0), ( 1,  1), ( 0, -2), ( 1, -2)],
        [( 0,  1), (-1,  1), (-1,  0), ( 0,  3), (-1,  3)],
    ],
    [
        [( 0, -1), ( 0,  0)],
        [(-1,  0), ( 0,  0)],
        [( 0,  1), ( 0,  0)],
        [( 1,  0), ( 0,  0)],
    ],
    [
        [( 0, -1), ( 1, -1), ( 1,  0), ( 0, -3), ( 1, -3)],
        [(-1,  1), ( 0,  1), ( 0,  0), (-1,  3), ( 0,  3)],
        [( 1,  0), ( 0,  0), ( 0,  1), ( 1, -2), ( 0, -2)],
        [( 0,  0), (-1,  0), (-1, -1), ( 0,  2), (-1,  2)],
    ],
);

/// Kick data for I pieces under TETRIO.  *Different* from other I kick data,
/// because TETRIO uses SRS+ kicks --- which are more symmetric, but not
/// rotationally inverted --- for I pieces.
#[rustfmt::skip]
pub static TETRIO_I: Kicks<5, 6> = Kicks::make(
    [
        [( 2, -2), ( 3, -2), ( 0, -2), ( 0, -3), ( 3,  0)],
        [(-2,  1), (-3,  1), ( 0,  1), (-3,  3), ( 0,  0)],
        [( 1, -1), ( 3, -1), ( 0, -1), ( 3,  0), ( 0, -3)],
        [(-1,  2), ( 0,  2), (-3,  2), ( 0,  0), (-3,  3)],
    ],
    [
        [( 0, -1), ( 0,  0), ( 1,  0), (-1,  0), ( 1, -1), (-1, -1)],
        [( 0,  1), ( 0,  0), (-1,  0), ( 1,  0), (-1,  1), ( 1,  1)],
        [(-1,  0), ( 0,  0), ( 0,  2), ( 0,  1), (-1,  2), (-1,  1)],
        [( 1,  0), ( 0,  0), ( 0,  2), ( 0,  1), ( 1,  2), ( 1,  1)],
    ],
    [
        [( 1, -2), ( 0, -2), ( 3, -2), ( 3, -3), ( 0,  0)],
        [(-2,  2), (-3,  2), ( 0,  2), (-3,  0), ( 0,  3)],
        [( 2, -1), ( 0, -1), ( 3, -1), ( 0,  0), ( 3, -3)],
        [(-1,  1), ( 0,  1), (-3,  1), ( 0,  3), (-3,  0)],
    ],
);
/// Kick data for J, L, S, T, and Z pieces under TETRIO.  Identical to
/// [`JSTRIS_JLSTZ`] but with four more kick offsets for half rotations.
///
/// [`JSTRIS_JLSTZ`]: JSTRIS_JLSTZ
#[rustfmt::skip]
pub static TETRIO_JLSTZ: Kicks<5, 6> = Kicks::make(
    [
        [( 1, -1), ( 0, -1), ( 0,  0), ( 1, -3), ( 0, -3)],
        [(-1,  0), ( 0,  0), ( 0, -1), (-1,  2), ( 0,  2)],
        [( 0,  0), ( 1,  0), ( 1,  1), ( 0, -2), ( 1, -2)],
        [( 0,  1), (-1,  1), (-1,  0), ( 0,  3), (-1,  3)],
    ],
    [
        [( 0, -1), ( 0,  0), ( 1,  0), (-1,  0), ( 1, -1), (-1, -1)],
        [(-1,  0), ( 0,  0), ( 0,  2), ( 0,  1), (-1,  2), (-1,  1)],
        [( 0,  1), ( 0,  0), (-1,  0), ( 1,  0), (-1,  1), ( 1,  1)],
        [( 1,  0), ( 0,  0), ( 0,  2), ( 0,  1), ( 1,  2), ( 1,  1)],
    ],
    [
        [( 0, -1), ( 1, -1), ( 1,  0), ( 0, -3), ( 1, -3)],
        [(-1,  1), ( 0,  1), ( 0,  0), (-1,  3), ( 0,  3)],
        [( 1,  0), ( 0,  0), ( 0,  1), ( 1, -2), ( 0, -2)],
        [( 0,  0), (-1,  0), (-1, -1), ( 0,  2), (-1,  2)],
    ],
);

impl Collision {
    /// Compute collision data for a single shape and orientation from the given
    /// mino coordinates.  The provided coordinates are for a piece at position
    /// (0, 0), and are specified by `(column, row)`, just like [`Piece`].
    ///
    /// [`Piece`]: crate::gameplay::Piece
    pub const fn make(minoes: [(u8, u8); 4]) -> Collision {
        let shifts = [
            minoes[0].0 + minoes[0].1 * 10,
            minoes[1].0 + minoes[1].1 * 10,
            minoes[2].0 + minoes[2].1 * 10,
            minoes[3].0 + minoes[3].1 * 10,
        ];

        let row_mask = !0
            & (FULL_10 >> minoes[0].0)
            & (FULL_10 >> minoes[1].0)
            & (FULL_10 >> minoes[2].0)
            & (FULL_10 >> minoes[3].0);

        let max_row = max4(minoes[0].1, minoes[1].1, minoes[2].1, minoes[3].1);

        Collision {
            shifts,
            mask: replicate_row(row_mask),
            placeable_shift: 24 + 10 * max_row,
        }
    }

    /// Find which positions on the board are viable for this shape and
    /// orientation.  (In other words, all positions at which a piece could be
    /// placed if it were teleported in. See [here](crate::vector#how).)
    pub fn viable(&self, board: Board) -> PVec {
        let collisions = 0
            | board.0 >> self.shifts[0]
            | board.0 >> self.shifts[1]
            | board.0 >> self.shifts[2]
            | board.0 >> self.shifts[3];

        PVec(!collisions & self.mask)
    }

    /// Find which positions are placeable for this shape and orientation.  This
    /// will cut off positions from the top, *possibly even ones in bounds*,
    /// because if a piece were placed there, it might peek out the top of the
    /// board.
    pub fn placeable(&self, reachable: PVec) -> PVec {
        let grounded = reachable.0 & !(reachable.0 << 10);
        PVec(grounded << self.placeable_shift >> self.placeable_shift)
    }
}

impl<const QUARTER: usize, const HALF: usize> Kicks<QUARTER, HALF> {
    pub const fn make(
        cw_offsets: [[(i8, i8); QUARTER]; 4],
        half_offsets: [[(i8, i8); HALF]; 4],
        ccw_offsets: [[(i8, i8); QUARTER]; 4],
    ) -> Self {
        let mut cw_rotates = [[0; QUARTER]; 4];
        let mut half_rotates = [[0; HALF]; 4];
        let mut ccw_rotates = [[0; QUARTER]; 4];

        let mut cw_masks = [[0; QUARTER]; 4];
        let mut half_masks = [[0; HALF]; 4];
        let mut ccw_masks = [[0; QUARTER]; 4];

        pub const fn make_one((cols, rows): (i8, i8)) -> (u8, u64) {
            debug_assert!(cols.abs() < 10);
            debug_assert!(rows.abs() < 4);

            let row_mask = shift_left_signed(FULL_10, cols) & FULL_10;
            let board_mask = shift_left_signed(replicate_row(row_mask), rows * 10) & FULL_60;
            let signed_shift = cols + rows * 10;

            ((signed_shift + 64) as u8 % 64, board_mask)
        }

        let mut i = 0;
        while i < 4 {
            let mut j = 0;
            while j < QUARTER {
                (cw_rotates[i][j], cw_masks[i][j]) = make_one(cw_offsets[i][j]);
                (ccw_rotates[i][j], ccw_masks[i][j]) = make_one(ccw_offsets[i][j]);
                j += 1;
            }

            j = 0;
            while j < HALF {
                (half_rotates[i][j], half_masks[i][j]) = make_one(half_offsets[i][j]);
                j += 1;
            }

            i += 1
        }

        Kicks {
            rotates: (cw_rotates, half_rotates, ccw_rotates),
            masks: (cw_masks, half_masks, ccw_masks),
        }
    }

    pub fn cw(&self, initial: Orientation, from: PVec, viable: PVec) -> PVec {
        Self::do_kicks(initial, from, viable, &self.rotates.0, &self.masks.0)
    }

    pub fn half(&self, initial: Orientation, from: PVec, viable: PVec) -> PVec {
        Self::do_kicks(initial, from, viable, &self.rotates.1, &self.masks.1)
    }

    pub fn ccw(&self, initial: Orientation, from: PVec, viable: PVec) -> PVec {
        Self::do_kicks(initial, from, viable, &self.rotates.2, &self.masks.2)
    }

    // TODO: Inline?
    fn do_kicks<const N: usize>(
        initial: Orientation,
        from: PVec,
        viable: PVec,
        rotates: &[[u8; N]; 4],
        masks: &[[u64; N]; 4],
    ) -> PVec {
        let rotates = rotates[initial as usize];
        let masks = masks[initial as usize];

        let mut from = from.0;
        let mut to = 0;
        let mask = viable.0;

        for i in 0..N {
            let kicked = from.rotate_left(rotates[i] as u32) & masks[i] & mask;
            from ^= kicked.rotate_right(rotates[i] as u32);
            to |= kicked;
        }

        PVec(to)
    }
}

impl std::fmt::Debug for PVec {
    /// This formatter prints position vectors as 6×10 boards.  This can't be
    /// directly typed back in to reproduce the vector, but it's often more
    /// useful.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\n")?;

        for row in (0..6).rev() {
            for col in 0..10 {
                if (self.0 & (1 << row * 10 + col)) != 0 {
                    write!(f, "█")?;
                } else {
                    write!(f, "░")?;
                }
            }
            write!(f, "\n")?;
        }
        write!(f, "\n")?;

        Ok(())
    }
}

// Various utility functions.

/// Calculate `n << by` where `by` can be negative.
const fn shift_left_signed(n: u64, by: i8) -> u64 {
    if by >= 0 {
        n << by
    } else {
        n >> (-by)
    }
}

/// Copy one row into all six rows.
const fn replicate_row(row: u64) -> u64 {
    debug_assert!(row == row & FULL_10);
    row | (row << 10) | (row << 20) | (row << 30) | (row << 40) | (row << 50)
}

/// The maximum of two numbers, as a `const fn`.
const fn max(a: u8, b: u8) -> u8 {
    if a > b {
        a
    } else {
        b
    }
}

/// The maximum of four numbers, as a `const fn`.
const fn max4(a: u8, b: u8, c: u8, d: u8) -> u8 {
    max(a, max(b, max(c, d)))
}

impl std::ops::BitAnd for PVec {
    type Output = PVec;
    fn bitand(self, rhs: PVec) -> PVec {
        PVec(self.0 & rhs.0)
    }
}
impl std::ops::BitOr for PVec {
    type Output = PVec;
    fn bitor(self, rhs: PVec) -> PVec {
        PVec(self.0 | rhs.0)
    }
}
impl std::ops::BitAndAssign for PVec {
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}
impl std::ops::BitOrAssign for PVec {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitAnd for Placements {
    type Output = Placements;

    fn bitand(self, rhs: Self) -> Self::Output {
        assert_eq!(self.shape, rhs.shape);
        assert_eq!(self.board, rhs.board); // TODO: `and` these together?
        Placements {
            positions: [
                self.positions[0] & rhs.positions[0],
                self.positions[1] & rhs.positions[1],
                self.positions[2] & rhs.positions[2],
                self.positions[3] & rhs.positions[3],
            ],
            ..self
        }
    }
}
impl std::ops::BitOr for Placements {
    type Output = Placements;

    fn bitor(self, rhs: Self) -> Self::Output {
        assert_eq!(self.shape, rhs.shape);
        assert_eq!(self.board, rhs.board); // TODO: `or` these together?
        Placements {
            positions: [
                self.positions[0] | rhs.positions[0],
                self.positions[1] | rhs.positions[1],
                self.positions[2] | rhs.positions[2],
                self.positions[3] | rhs.positions[3],
            ],
            ..self
        }
    }
}
