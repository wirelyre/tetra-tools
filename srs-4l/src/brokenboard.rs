use std::collections::HashSet;

use bitvec::prelude::*;
use smallvec::SmallVec;

use crate::{
    gameplay::{Board, Orientation, Physics, Piece, Shape},
    queue::Queue,
    vector::Placements,
};

/// A [board] which keeps track of the individual [pieces] placed in it.
///
/// Unlike in a regular board, cleared lines stay in place.  After a line has
/// been cleared, later placed pieces are "broken" across the empty row.
///
/// Broken boards are directly comparable via `Eq`, `Ord`, and `Hash`.  They
/// have a stable identity formed by the individual broken pieces inside them.
/// This identity is not affected by the order in which pieces were placed.
///
/// [board]: Board
/// [pieces]: BrokenPiece
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BrokenBoard {
    pub board: Board,
    /// Bit vector of which rows have been cleared.
    pub cleared_rows: u8,
    pub pieces: SmallVec<[BrokenPiece; 10]>,
}

/// A piece in a [`BrokenBoard`].
///
/// Might span several noncontiguous lines.  Sorted by the lowest filled mino.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BrokenPiece {
    /// Index of the lowest cell filled by this piece.
    pub low_mino: u8,
    pub shape: Shape,
    /// Canonical orientation of this piece.
    pub orientation: Orientation,
    /// Bit vector of which rows contain at least one of this piece's minoes.  A
    /// set bit indicates that the piece has a mino in the row; an unset bit
    /// indicates an empty row.
    pub rows: u8,
}

impl BrokenBoard {
    pub fn empty() -> Self {
        BrokenBoard {
            board: Board::empty(),
            cleared_rows: 0,
            pieces: SmallVec::new(),
        }
    }

    pub fn from_garbage(garbage: u64) -> Self {
        let mut new = BrokenBoard {
            board: Board(0),
            cleared_rows: 0,
            pieces: SmallVec::new(),
        };

        let mut complete_lines = 0;
        let mut complete_lines_shift = 0;

        for row in (0..4).rev() {
            let this_line = (garbage >> (row * 10)) & 0b1111111111;

            if this_line == 0b1111111111 {
                complete_lines <<= 10;
                complete_lines |= 0b1111111111;
                complete_lines_shift += 10;
                new.cleared_rows |= 1 << row;
            } else {
                new.board.0 <<= 10;
                new.board.0 |= this_line;
            }
        }

        new.board.0 <<= complete_lines_shift;
        new.board.0 |= complete_lines;

        new
    }

    pub fn to_broken_bitboard(&self) -> Board {
        let mut old = self.board.0;
        let mut new = 0;

        for row in (0..4).rev() {
            let full = (self.cleared_rows & (1 << row)) != 0;

            let new_row = if full {
                old >>= 10;
                0b1111111111
            } else {
                (old >> (10 * row)) & 0b1111111111
            };

            new <<= 10;
            new |= new_row;
        }

        Board(new)
    }

    pub fn place(&self, piece: Piece) -> Self {
        let mut new = BrokenBoard {
            board: piece.place(self.board),
            cleared_rows: 0,
            pieces: self.pieces.clone(),
        };

        let cleared_count = self.cleared_rows.count_ones();

        let minoes = piece.as_board().0 >> (cleared_count * 10);
        let field = (self.board.0 >> (cleared_count * 10)) | minoes;

        let mut row_mask = 0b1111111111;
        let mut rows = 0;

        for row in 0..=3 {
            let row_bit = 1 << row;

            if self.cleared_rows & row_bit != 0 {
                new.cleared_rows |= row_bit;
            } else {
                if minoes & row_mask != 0 {
                    rows |= row_bit;
                }
                if field & row_mask == row_mask {
                    new.cleared_rows |= row_bit;
                }

                row_mask <<= 10;
            }
        }

        let low_mino = minoes.trailing_zeros() % 10 + rows.trailing_zeros() * 10;

        new.pieces.push(BrokenPiece {
            low_mino: low_mino as u8,
            shape: piece.shape,
            orientation: piece.orientation.canonical(piece.shape),
            rows,
        });
        new.pieces.sort_unstable();

        new
    }

    pub fn encode(&self) -> BitVec {
        let mut bv = BitVec::new();

        // magic number, leaves room for larger boards in the future
        let max_rows: u8 = 4;
        bv.extend_from_bitslice(max_rows.view_bits::<Lsb0>());

        // board
        // must be split because `u64: BitStore` only if `pointer_width = 64`
        let low = self.board.0 as u32;
        let high = (self.board.0 >> 32) as u32;
        bv.extend_from_bitslice(low.view_bits::<Lsb0>());
        bv.extend_from_bitslice(&high.view_bits::<Lsb0>()[..8]);

        // cleared rows
        bv.extend_from_bitslice(&self.cleared_rows.view_bits::<Lsb0>()[..4]);

        // pieces
        for piece in &self.pieces {
            bv.extend_from_bitslice(&piece.low_mino.view_bits::<Lsb0>()[..6]); // low_mino < 40
            bv.extend_from_bitslice(&(piece.shape as u8).view_bits::<Lsb0>()[..3]); // 7 shapes
            bv.extend_from_bitslice(&(piece.orientation as u8).view_bits::<Lsb0>()[..2]); // 4 orientations
            bv.extend_from_bitslice(&piece.rows.view_bits::<Lsb0>()[..4]); // 4 rows
        }

        bv
    }

    pub fn decode(mut encoded: &BitSlice) -> Option<Self> {
        if encoded.len() < 52 || encoded.len() > 202 {
            return None;
        }

        let mut new = BrokenBoard::empty();

        if encoded[..8].load_le::<u8>() != 4 {
            // wrong magic
            return None;
        }
        encoded = &encoded[8..];

        new.board = Board(encoded[..40].load_le());
        encoded = &encoded[40..];

        new.cleared_rows = encoded[..4].load_le();
        encoded = &encoded[4..];

        while encoded.len() != 0 {
            if encoded.len() < 15 {
                // not long enough for a piece
                return None;
            }

            new.pieces.push(BrokenPiece {
                low_mino: encoded[..6].load_le(),
                shape: Shape::try_from(encoded[6..9].load_le())?,
                orientation: Orientation::try_from(encoded[9..11].load_le())?,
                rows: encoded[11..15].load_le(),
            });

            encoded = &encoded[15..];
        }

        if new.is_valid() {
            Some(new)
        } else {
            None
        }
    }

    pub fn is_valid(&self) -> bool {
        // full lines are at bottom
        if self.board != BrokenBoard::from_garbage(self.board.0).board {
            return false;
        }

        // cleared row count is correct
        let full_line_count = (0..4)
            .map(|i| 0b1111111111 << (10 * i))
            .take_while(|&row| self.board.0 & row == row)
            .count() as u32;
        if full_line_count != self.cleared_rows.count_ones() {
            return false;
        }

        let mut board = self.to_broken_bitboard().0;

        // pieces are contained in the board, and do not overlap
        for piece in &self.pieces {
            let piece_board = piece.board().0;
            if board & piece_board != piece_board {
                return false;
            }
            board ^= piece_board;
        }

        // it's okay if `board != 0`; that means there was initial garbage

        true
    }

    /// Determine if a piece can be placed in this board.
    ///
    /// If `Some(p)` is returned, then:
    ///   1. `!self.pieces.contains(&piece)`
    ///   2. `p.can_place(self.board)`
    ///   3. `self.place(p).pieces.contains(&piece)`
    pub fn placeable(&self, piece: BrokenPiece) -> Option<Piece> {
        if self.pieces.contains(&piece) {
            return None;
        }

        // precondition: this piece is compatible with this board
        debug_assert_eq!(self.cleared_rows & piece.rows, 0);

        // bitwise utility functions
        let below_top_1 = |n: u8| (1u8 << (8 - n.leading_zeros())) - 1;
        let above_bottom_1 = |n: u8| !((1u8 << n.trailing_zeros()) - 1);

        // Which rows, if any, must be cleared for this piece to work?
        // That is, which rows split the piece?  They must be cleared already.
        let required_clear = below_top_1(piece.rows) & above_bottom_1(piece.rows) & !piece.rows;
        if required_clear & self.cleared_rows != required_clear {
            return None;
        }

        // offset of left bound of piece
        let bump_col = crate::gameplay::PIECE_SHAPES[piece.shape as usize]
            [piece.orientation as usize]
            .trailing_zeros();

        // Which lines have been stashed at the bottom of the board?
        // The coordinates of `piece` already account for lines below `piece.low_mino`.
        // How many cleared lines lie above the lowest line this piece inhabits?
        let bump_row = (self.cleared_rows & above_bottom_1(piece.rows)).count_ones();

        let p = Piece {
            shape: piece.shape,
            col: (piece.low_mino % 10) as i8 - bump_col as i8,
            row: (piece.low_mino / 10) as i8 + bump_row as i8,
            orientation: piece.orientation,
        };
        if !p.can_place(self.board) {
            return None;
        }

        // a complicated assertion:
        //   - this piece could be legally placed if teleported into position
        //   - when placed, it will break back up into `piece`
        // these should be ensured by the logic above, but it's good to check
        debug_assert!(self.place(p).pieces.contains(&piece));

        Some(p)
    }

    /// Run a search to find all queues that can produce this board without
    /// holding.
    pub fn supporting_queues(&self, physics: Physics) -> Vec<Queue> {
        let mut garbage = self.to_broken_bitboard().0;

        for &piece in &self.pieces {
            garbage ^= piece.board().0;
        }

        let mut prev = HashSet::new();
        prev.insert((BrokenBoard::from_garbage(garbage), Queue::empty()));

        for _ in 0..self.pieces.len() {
            let mut next = HashSet::new();

            for (board, queue) in prev {
                let mut placeable: Vec<Piece> = self
                    .pieces
                    .iter()
                    .filter_map(|&p| board.placeable(p))
                    .collect();

                for shape in Shape::ALL {
                    if !placeable.iter().any(|p| p.shape == shape) {
                        continue;
                    }

                    for (piece, _) in Placements::place(board.board, shape, physics).canonical() {
                        if placeable.contains(&piece) {
                            let pair = (board.place(piece), queue.push_last(shape));

                            next.insert(pair);

                            let index = placeable.iter().position(|p| p == &piece).unwrap();
                            placeable.swap_remove(index);

                            if !placeable.iter().any(|p| p.shape == shape) {
                                break;
                            }
                        }
                    }
                }
            }

            prev = next;
        }

        prev.iter().map(|(_, queue)| *queue).collect()
    }
}

impl BrokenPiece {
    /// The bitboard corresponding to this piece.
    ///
    /// The returned board is probably not contiguous, and is really only useful
    /// for locating the minoes.
    pub fn board(self) -> Board {
        let connected =
            crate::gameplay::PIECE_SHAPES[self.shape as usize][self.orientation as usize];
        let mut connected = connected >> connected.trailing_zeros() << (self.low_mino % 10);

        let mut broken = 0;

        for row in 0..=3 {
            if (1 << row) & self.rows != 0 {
                broken |= (0b1111111111 & connected) << (row * 10);
                connected >>= 10;
            }
        }

        assert_eq!(connected, 0);

        Board(broken)
    }
}
