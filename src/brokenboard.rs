use smallvec::SmallVec;

use crate::gameplay::{Board, Piece, Rotation, Shape};

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
    /// Canonical rotation of this piece.
    pub rotation: Rotation,
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
            rotation: piece.rotation.canonical(piece.shape),
            rows,
        });
        new.pieces.sort_unstable();

        new
    }
}

impl BrokenPiece {
    /// The bitboard corresponding to this piece.
    ///
    /// The returned board is probably not contiguous, and is really only useful
    /// for locating the minoes.
    pub fn board(self) -> Board {
        let connected = crate::gameplay::PIECE_SHAPES[self.shape as usize][self.rotation as usize];
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
