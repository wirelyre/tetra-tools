//! Vector implementation of 4-line SRS.  Computes all piece placements in
//! parallel.

use crate::gameplay::{Board, Orientation, Piece, Shape};

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PVec(pub u64);

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Placements {
    pub shape: Shape,
    pub board: Board,
    pub positions: [PVec; 4],
}

impl Placements {
    pub fn place(board: Board, shape: Shape) -> Self {
        use Orientation::*;

        let collision = &COLLISION[shape as usize];

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

    pub fn contains(&self, piece: Piece) -> bool {
        self.shape == piece.shape
            && self.positions[piece.orientation as usize].contains(piece.col, piece.row)
    }

    pub fn remove(&mut self, piece: Piece) -> bool {
        self.shape == piece.shape
            && self.positions[piece.orientation as usize].remove(piece.col, piece.row)
    }
}

pub struct PlacementMachine {
    shape: Shape,
    viable: [PVec; 4],
    reachable: [PVec; 4],
    dirty: [bool; 4],
}

impl PlacementMachine {
    fn any_dirty(&self) -> bool {
        self.dirty.iter().any(|b| *b)
    }

    fn step(&mut self, o: Orientation) {
        let ccw = o.ccw() as usize;
        let this = o as usize;
        let cw = o.cw() as usize;

        let kicks = KICKS[self.shape as usize];

        if self.dirty[this] {
            self.reachable[this] = self.reachable[this].flood_fill(self.viable[this]);

            let cw_more = kicks[this].kick_cw(self.reachable[this], self.viable[cw]);
            if (self.reachable[cw] & cw_more) != cw_more {
                self.reachable[cw] |= cw_more;
                self.dirty[cw] = true;
            }

            let ccw_more = kicks[ccw].kick_ccw(self.reachable[this], self.viable[ccw]);
            if (self.reachable[ccw] & ccw_more) != ccw_more {
                self.reachable[ccw] |= ccw_more;
                self.dirty[ccw] = true;
            }

            self.dirty[this] = false;
        }
    }

    fn placeable(&self, o: Orientation) -> PVec {
        COLLISION[self.shape as usize][o as usize].placeable(self.reachable[o as usize])
    }
}

impl Iterator for Placements {
    type Item = (Piece, Board);

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.positions.iter().copied().map(PVec::count).sum();
        (len, Some(len))
    }
}

impl DoubleEndedIterator for Placements {
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

pub const FULL_10: u64 = 0x3FF;
pub const FULL_60: u64 = 0xFFFFFFFFFFFFFFF;

pub const LEFT_50: PVec = PVec(replicate_row(0b0111111111));
pub const RIGHT_50: PVec = PVec(replicate_row(0b1111111110));

pub const SPAWN: PVec = PVec((FULL_10 << 50) | (FULL_10 << 40));

impl PVec {
    pub const fn count(self) -> usize {
        self.0.count_ones() as usize
    }

    #[must_use]
    pub fn or_down(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 >> 10 & viable.0))
    }

    #[must_use]
    pub fn or_left(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 >> 1 & LEFT_50.0 & viable.0))
    }

    #[must_use]
    pub fn or_right(self, viable: PVec) -> PVec {
        PVec(self.0 | (self.0 << 1 & RIGHT_50.0 & viable.0))
    }

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

    pub const fn contains(self, col: i8, row: i8) -> bool {
        self.0 & (1 << (col + row * 10)) != 0
    }

    pub fn remove(&mut self, col: i8, row: i8) -> bool {
        let mask = 1 << (col + row * 10);

        if self.0 & mask == 0 {
            return false;
        }

        self.0 ^= mask;
        true
    }
}

pub struct Collision {
    shifts: [u8; 4],
    mask: u64,
    placeable_shift: u8,
}

pub struct Kicks(pub [u8; 5], pub [u64; 5]);

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

pub static KICKS: [&[Kicks; 4]; 7] = [
    &I_KICKS,     /* I */
    &JLSTZ_KICKS, /* J */
    &JLSTZ_KICKS, /* L */
    &O_KICKS,     /* O */
    &JLSTZ_KICKS, /* S */
    &JLSTZ_KICKS, /* T */
    &JLSTZ_KICKS, /* Z */
];

pub static I_KICKS: [Kicks; 4] = [
    Kicks::make([(2, -2), (0, -2), (3, -2), (0, -3), (3, 0)]),
    Kicks::make([(-2, 1), (-3, 1), (0, 1), (-3, 3), (0, 0)]),
    Kicks::make([(1, -1), (3, -1), (0, -1), (3, 0), (0, -3)]),
    Kicks::make([(-1, 2), (0, 2), (-3, 2), (0, 0), (-3, 3)]),
];

pub static JLSTZ_KICKS: [Kicks; 4] = [
    Kicks::make([(1, -1), (0, -1), (0, 0), (1, -3), (0, -3)]),
    Kicks::make([(-1, 0), (0, 0), (0, -1), (-1, 2), (0, 2)]),
    Kicks::make([(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)]),
    Kicks::make([(0, 1), (-1, 1), (-1, 0), (0, 3), (-1, 3)]),
];

pub static O_KICKS: [Kicks; 4] = [
    Kicks::make([(0, 0); 5]),
    Kicks::make([(0, 0); 5]),
    Kicks::make([(0, 0); 5]),
    Kicks::make([(0, 0); 5]),
];

impl Collision {
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

    pub fn viable(&self, board: Board) -> PVec {
        let collisions = 0
            | board.0 >> self.shifts[0]
            | board.0 >> self.shifts[1]
            | board.0 >> self.shifts[2]
            | board.0 >> self.shifts[3];

        PVec(!collisions & self.mask)
    }

    pub fn placeable(&self, reachable: PVec) -> PVec {
        let grounded = reachable.0 & !(reachable.0 << 10);
        PVec(grounded << self.placeable_shift >> self.placeable_shift)
    }
}

impl Kicks {
    pub const fn make(offsets: [(i8, i8); 5]) -> Kicks {
        pub const fn make_one(cols: i8, rows: i8) -> (u8, u64) {
            debug_assert!(cols.abs() < 10);
            debug_assert!(rows.abs() < 4);

            let row_mask = shift_left_signed(FULL_10, cols) & FULL_10;
            let board_mask = shift_left_signed(replicate_row(row_mask), rows * 10) & FULL_60;
            let signed_shift = cols + rows * 10;

            ((signed_shift + 64) as u8 % 64, board_mask)
        }

        let kick0 = make_one(offsets[0].0, offsets[0].1);
        let kick1 = make_one(offsets[1].0, offsets[1].1);
        let kick2 = make_one(offsets[2].0, offsets[2].1);
        let kick3 = make_one(offsets[3].0, offsets[3].1);
        let kick4 = make_one(offsets[4].0, offsets[4].1);

        Kicks(
            [kick0.0, kick1.0, kick2.0, kick3.0, kick4.0],
            [kick0.1, kick1.1, kick2.1, kick3.1, kick4.1],
        )
    }

    pub fn kick_cw(&self, start: PVec, cw_viable: PVec) -> PVec {
        const fn kick(kicks: &Kicks, num: usize, from: u64, to: u64, mask: u64) -> (u64, u64) {
            let kicked = from.rotate_left(kicks.0[num] as u32) & kicks.1[num] & mask;
            (from ^ kicked.rotate_right(kicks.0[num] as u32), to | kicked)
        }

        let from = start.0;
        let to = 0;
        let mask = cw_viable.0;

        let (from, to) = kick(self, 0, from, to, mask);
        let (from, to) = kick(self, 1, from, to, mask);
        let (from, to) = kick(self, 2, from, to, mask);
        let (from, to) = kick(self, 3, from, to, mask);
        let (_from, to) = kick(self, 4, from, to, mask);

        PVec(to)
    }

    pub fn kick_ccw(&self, start: PVec, ccw_viable: PVec) -> PVec {
        const fn kick(kicks: &Kicks, num: usize, from: u64, to: u64, mask: u64) -> (u64, u64) {
            let kicked = (from & kicks.1[num]).rotate_right(kicks.0[num] as u32) & mask;
            (from ^ kicked.rotate_left(kicks.0[num] as u32), to | kicked)
        }

        let from = start.0;
        let to = 0;
        let mask = ccw_viable.0;

        let (from, to) = kick(self, 0, from, to, mask);
        let (from, to) = kick(self, 1, from, to, mask);
        let (from, to) = kick(self, 2, from, to, mask);
        let (from, to) = kick(self, 3, from, to, mask);
        let (_from, to) = kick(self, 4, from, to, mask);

        PVec(to)
    }
}

impl std::fmt::Debug for PVec {
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

/// Calculate `n << by` where `by` can be negative.
const fn shift_left_signed(n: u64, by: i8) -> u64 {
    if by >= 0 {
        n << by
    } else {
        n >> (-by)
    }
}

const fn replicate_row(row: u64) -> u64 {
    debug_assert!(row == row & FULL_10);
    row | (row << 10) | (row << 20) | (row << 30) | (row << 40) | (row << 50)
}

const fn max(a: u8, b: u8) -> u8 {
    if a > b {
        a
    } else {
        b
    }
}

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
