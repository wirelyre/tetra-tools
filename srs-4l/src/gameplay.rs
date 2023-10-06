//! Game data types and physics.

/// A packed bit representation of a board.
///
/// Bit 0 (the least significant bit) represents the bottom left of the board.
/// Bit 1 represents the cell immediately to the right. After bit 9, bit 10
/// wraps around to the leftmost cell one row upwards.
///
/// Although 64 bits are usable, valid boards only ever have the bottom 40 bits
/// set.  The top 24 bits are always clear.
///
/// This type is `Copy` because it is intended to be cheap to use.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Board(pub u64);

/// A piece, stored as [`shape`], coordinates, and [`orientation`].
///
/// Coordinates are measured from the bottom left of the piece's bounding box.
/// `col` increases to the right; `row` increases upwards.  This is different
/// from typical coordinates, which often have space to the left or bottom in
/// certain orientations.
///
/// `row` can be larger than 3; this means that the piece is beyond the top of
/// the bottom four rows of the board.
///
/// This type is `Copy` because it is intended to be cheap to use.  This means
/// that all methods produce *new* pieces.  Methods which take and return values
/// of the same type, whose results might accidentally be ignored, are marked
/// `must_use`.
///
/// ## Valid pieces
///
/// Valid pieces are completely in bounds: `col` and `row` are positive, despite
/// being signed; and `col` is never so large that the piece would extend past
/// the right edge of the board in the current orientation.
///
/// The methods on this type will only produce valid pieces.
///
/// [`shape`]:       Shape
/// [`orientation`]: Orientation
///
/// # Rotation system
///
/// This program uses [variants of the Super Rotation System (SRS)][Physics].
/// When a piece rotates, if it collides with something on the board, it doesn't
/// give up right away.  Instead, it tries moving around a bit to see if it fits
/// somewhere else.
///
/// When a piece has to try more than one position before it succeeds, that's
/// called a *kick*.  In SRS, five positions (the initial position plus four
/// more) are tried.  Which positions are tried?  That depends on the piece, its
/// current orientation, and the direction it's rotating.  See
/// [here](https://harddrop.com/wiki/SRS) for the details.
///
/// However, the coordinate system here is different from usual, because
/// coordinates are measured from the bottom left of the piece's bounding box.
/// (This keeps the numbers positive, which simplifies some of the math.)
///
/// To compensate, we alter the kick data so that the *first* checked position
/// is shifted too --- equivalent to the usual rotation --- and the other kicks
/// are shifted by the same amount.
///
/// We actually only store clockwise and half-rotation kick data.
/// Counter-clockwise kicks are exact mirrors of clockwise kicks.
///
/// # SRS
///
/// Methods on this struct use SRS.  For other rotation systems, use the
/// [`vector`] module.
///
/// [`vector`]: crate::vector
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Piece {
    pub shape: Shape,
    pub col: i8,
    pub row: i8,
    pub orientation: Orientation,
}

/// Each of the conventional single-letter names of tetrominoes.
///
/// The `u8` numeric representation is used as an index sometimes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Shape {
    I,
    J,
    L,
    O,
    S,
    T,
    Z,
}

/// Each possible orientation of tetrominoes.
///
/// The `u8` numeric representation is used as an index sometimes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Orientation {
    /// The initial orientation when a piece spawns --- minoes tend to be above
    /// a piece's center of rotation.
    North,
    /// One 90° clockwise rotation from spawn --- minoes tend to be right of a
    /// piece's center of rotation.
    East,
    /// One 180° half rotation from spawn --- minoes tend to be below a piece's
    /// center of rotation.
    South,
    /// One 90° counter-clockwise rotation from spawn --- minoes tend to be left
    /// of a piece's center of rotation.
    West,
}

/// Different rotation systems, distinguished by their handling of half
/// rotations.
///
/// Jstris is strictly additive over SRS:  Every placement in SRS is valid in
/// Jstris.  TETRIO is not:  Some placements in SRS are invalid in TETRIO, and
/// vice versa.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Physics {
    /// Super Rotation System.  Uses only quarter rotations, where each rotation
    /// checks five kick offsets.
    SRS,
    /// System used by Jstris.  Uses SRS quarter rotations and adds two kick
    /// offsets for half rotations.
    Jstris,
    /// System used by TETRIO.  Uses SRS+ quarter rotations (which are SRS with
    /// I-piece quarter rotations modified to be more intuitive) and adds six
    /// kick offsets for half rotations.
    Tetrio,
}

impl Board {
    /// Create an empty board.
    pub fn empty() -> Board {
        Board(0)
    }

    /// Check whether the cell at the given row and column is set.
    ///
    /// Requires that 0 &le; `col` &le; 9 and 0 &le; `row` &le; 3.
    pub fn get(self, row: i8, col: i8) -> bool {
        assert!(col >= 0);
        assert!(col <= 9);
        assert!(row >= 0);
        assert!(row <= 3);

        let mask = 1 << (row * 10 + col);
        (self.0 & mask) != 0
    }

    /// Check whether the board has a cell that cannot be filled.
    ///
    /// If the two cells to the left and right of an empty cell are both full
    /// (or if one side is a wall and the other is a filled cell), filling the
    /// cell will require filling some other cell in the column.
    ///
    /// If all empty cells in a column are bounded on the left and right, the
    /// only way to fill them is to use a vertical I piece to fill the entire
    /// column.  This is true even if the lines are reordered, which happens
    /// when lines are cleared.  If the column has at least one full cell, it
    /// will never be possible to place a vertical I piece.
    ///
    /// This method uses subtle bit magic to do the check.
    ///
    /// This check saves a lot of time by culling unusable boards early.
    pub fn has_isolated_cell(self) -> bool {
        // Combine rows in two ways:
        //   Is the column full?
        //   Is the column non-empty?
        // These are 10-bit vectors, where each bit represents a column.
        // (The top bits of `not_empty` are garbage, but this is fixed later.)

        let full = (self.0 >> 30) & (self.0 >> 20) & (self.0 >> 10) & (self.0 >> 0);
        let not_empty = (self.0 >> 30) | (self.0 >> 20) | (self.0 >> 10) | (self.0 >> 0);

        let bounded = {
            // A cell is left-bounded if the cell one bit down is full,
            //   or if it's on the left edge.
            // Bits wrap around the edge, but they are always on after the "or" anyway.
            // (Bit 40 is garbage, but this is fixed shortly.)

            let left_bounded = (self.0 << 1) | 0b0000000001_0000000001_0000000001_0000000001;
            let right_bounded = (self.0 >> 1) | 0b1000000000_1000000000_1000000000_1000000000;

            // Is each cell either full, or both left- and right-bounded?
            // Bit 40 of `right_bounded` is 0, so bits 40-63 of the result are
            //   correctly clear.

            let bounded_cells = (left_bounded & right_bounded) | self.0;

            // Combine boundedness into a 10-bit vector.
            // Bits 10-63 of the result will be clear
            //   because bits 10-63 of `(bounded_cells >> 30)` are clear.
            (bounded_cells >> 30)
                & (bounded_cells >> 20)
                & (bounded_cells >> 10)
                & (bounded_cells >> 0)
        };

        // Is any column:
        //   - not empty, and
        //   - not full, and
        //   - each empty cell is left- and right-bounded?
        // If any of bits 0-9 are set, then some column has the property
        //   and the board is unusable.

        // Bits 10-63 of `not_empty` and `!full` are garbage.
        // But bits 10-63 of `bounded` are clear, so the garbage is cleared.

        (not_empty & !full & bounded) != 0
    }

    /// Check whether the board has a disconnected section that cannot be
    /// exactly filled.
    ///
    /// When a sequence of pieces are placed, the number of cells they fill is a
    /// multiple of four.
    ///
    /// So if a section of empty cells is disconnected from the rest of the
    /// board (that is, the empty cells will never touch any other cells in
    /// another section), but the number of empty cells is *not* a multiple of
    /// four, then no sequence of pieces can fill the section.  In this case,
    /// the board is impossible to fill.
    ///
    /// Similarly to [`has_isolated_cell`], this method finds disconnected
    /// sections by comparing two adjacent columns.  If, in two adjacent
    /// columns, there is at least one cell in each row, then all empty cells
    /// from the left column leftwards are permanently disconnected from all
    /// empty cells from the right column rightwards.  It is never possible to
    /// reach an empty cell in the right column from an empty cell in the left
    /// column.
    ///
    /// This check saves an enormous amount of time by culling unusable boards
    /// early.
    ///
    /// [`has_isolated_cell`]: Board::has_isolated_cell
    pub fn has_imbalanced_split(self) -> bool {
        const COL_0: u64 = 0b1_0000000001_0000000001_0000000001;
        const COL_1: u64 = COL_0 << 1;
        const COL_2: u64 = COL_0 << 2;
        const COL_3: u64 = COL_0 << 3;
        const COL_4: u64 = COL_0 << 4;
        const COL_5: u64 = COL_0 << 5;
        const COL_6: u64 = COL_0 << 6;
        const COL_7: u64 = COL_0 << 7;

        const LEFT_0: u64 = COL_0;
        const LEFT_1: u64 = LEFT_0 | COL_1;
        const LEFT_2: u64 = LEFT_1 | COL_2;
        const LEFT_3: u64 = LEFT_2 | COL_3;
        const LEFT_4: u64 = LEFT_3 | COL_4;
        const LEFT_5: u64 = LEFT_4 | COL_5;
        const LEFT_6: u64 = LEFT_5 | COL_6;
        const LEFT_7: u64 = LEFT_6 | COL_7;

        fn check_col(board: Board, col_mask: u64, left_mask: u64) -> bool {
            // "Or" this column and the next column together.
            // Is there a filled cell in each row?

            if (board.0 | (board.0 >> 1)) & col_mask == col_mask {
                // There is a left section of the board containing all cells to
                // the left, as well as all cells *in this column*.
                let left = board.0 & left_mask;

                // This is a trick to skip a little logic.  In any combination
                // of columns, the *total* number of cells is a multiple of 4.
                // So the number of empty cells is a multiple of 4
                // if and only if the number of filled cells is a multiple of 4.

                // total ≡ empty + filled ≡ 0 (mod 4)
                // empty ≡ -filled
                // empty ≡ 0  ⟺  filled ≡ 0

                if left.count_ones() % 4 != 0 {
                    return true;
                }
            }

            false
        }

        // If columns 1-7 are checked, also checking 0 and 8 is equivalent to
        // also checking `has_isolated_cell`.

        // `has_isolated_cell` is faster.

        false
            || check_col(self, COL_1, LEFT_1)
            || check_col(self, COL_2, LEFT_2)
            || check_col(self, COL_3, LEFT_3)
            || check_col(self, COL_4, LEFT_4)
            || check_col(self, COL_5, LEFT_5)
            || check_col(self, COL_6, LEFT_6)
            || check_col(self, COL_7, LEFT_7)
    }
}

impl Piece {
    /// Create a new piece of the given shape.
    ///
    /// The new piece spawns just above the 4&times;10 board, on the left side.
    /// Since valid [boards] only use the bottom 40 bits, this new piece is valid in every board.
    ///
    /// [boards]: Board
    pub fn new(shape: Shape) -> Piece {
        Piece {
            shape,
            col: 0,
            row: 4,
            orientation: Orientation::North,
        }
    }

    /// Pack a piece into a 16-bit number.
    ///
    /// The number returned will be strictly less than 0x4000 = 16384.
    pub fn pack(self) -> u16 {
        // 0x3000 -> orientation (only 4 possibilities, so only 2 bits)
        // 0x0F00 -> shape
        // 0x00F0 -> column
        // 0x000F -> row
        ((self.orientation as u16) << 12)
            | ((self.shape as u16) << 8)
            | ((self.col as u16) << 4)
            | ((self.row as u16) << 0)
    }

    /// Unpack a number from [`pack`] into a piece.
    ///
    /// Only numbers from [`pack`] should be used, otherwise the piece might not
    /// be valid.   In debug mode, the piece is checked to make sure it's valid.
    ///
    /// [`pack`]: Piece::pack
    pub fn unpack(val: u16) -> Piece {
        let orientation = match (val & 0xF000) >> 12 {
            0 => Orientation::North,
            1 => Orientation::East,
            2 => Orientation::South,
            3 => Orientation::West,
            _ => unreachable!("invalid packed orientation"),
        };
        let kind = match (val & 0x0F00) >> 8 {
            0 => Shape::I,
            1 => Shape::J,
            2 => Shape::L,
            3 => Shape::O,
            4 => Shape::S,
            5 => Shape::T,
            6 => Shape::Z,
            _ => unreachable!("invalid packed kind"),
        };
        let col = ((val & 0x00F0) >> 4) as i8;
        let row = ((val & 0x000F) >> 4) as i8;

        let val = Piece {
            shape: kind,
            col,
            row,
            orientation,
        };
        debug_assert!(val.in_bounds());

        val
    }

    /// Convert a piece into a board, where the minoes of the piece are set on
    /// the board.
    ///
    /// In order to make sure that the board is valid, only minoes in the bottom
    /// four rows are kept.  Other minoes are cut off.
    ///
    /// This is not the same as [placing] a piece into an empty board!  Placing
    /// a piece requires that the piece is resting on a filled cell or the
    /// bottom of the board.  But `as_board` can make a board with floating
    /// minoes!
    ///
    /// [placing]: Piece::place
    pub fn as_board(self) -> Board {
        Board(self.as_bits() & BOARD_MASK)
    }

    /// Convert a piece into a bit board.  Exactly like [`as_board`], except
    /// without cutting off minoes above the four bottom rows.
    ///
    /// This is used internally when we either don't care about the upper bits,
    /// or when we actually *want* to look at the upper bits, like in
    /// [`can_place`].
    ///
    /// [`as_board`]:  Piece::as_board
    /// [`can_place`]: Piece::can_place
    fn as_bits(self) -> u64 {
        let shift = self.row * 10 + self.col;
        PIECE_SHAPES[self.shape as usize][self.orientation as usize] << shift
    }

    /// Check whether a piece collides with any filled cells on the board.
    fn collides_in(self, board: Board) -> bool {
        (self.as_bits() & board.0) != 0
    }

    /// Check whether a piece can be placed in the board.
    ///
    /// The piece must be:
    ///
    /// 1. Fully in bounds
    /// 2. Fully in the bottom four rows
    /// 3. Resting on a filled cell or the bottom of the board
    pub fn can_place(self, board: Board) -> bool {
        let bits = self.as_bits();
        ((bits & BOARD_MASK) != 0) && ((bits & !BOARD_MASK) == 0) && self.down(board) == self
    }

    /// Place a piece into the board, and move full lines to the bottom of the
    /// board.
    ///
    /// The piece must be:
    ///
    /// 1. Fully in bounds
    /// 2. Resting on a filled cell or the bottom of the board
    /// 3. Not overlapping any filled cell in the board
    ///
    /// In debug mode, those requirements are checked.
    ///
    /// Any full lines in the resulting board are shifted to the bottom of the
    /// board.  This is like clearing lines, but also keeps track of how many
    /// lines have been cleared on the board already.
    #[must_use]
    pub fn place(self, board: Board) -> Board {
        debug_assert!(self.can_place(board));
        debug_assert!((board.0 & self.as_bits()) == 0);

        let mut unordered_board = board.0 | self.as_bits();

        let mut ordered_board = 0;
        let mut complete_lines = 0;
        let mut complete_lines_shift = 0;

        for _ in 0..4 {
            let this_line = (unordered_board >> 30) & 0b1111111111;
            unordered_board <<= 10;

            if this_line == 0b1111111111 {
                complete_lines <<= 10;
                complete_lines |= this_line;
                complete_lines_shift += 10;
            } else {
                ordered_board <<= 10;
                ordered_board |= this_line;
            }
        }

        ordered_board <<= complete_lines_shift;
        ordered_board |= complete_lines;

        Board(ordered_board)
    }

    /// Shift a piece left.  If impossible, returns the piece unchanged.
    #[must_use]
    pub fn left(self, board: Board) -> Piece {
        let mut new = self;
        new.col -= 1;

        if (new.col < 0) || new.collides_in(board) {
            self
        } else {
            new
        }
    }

    /// Shift a piece right.  If impossible, returns the piece unchanged.
    #[must_use]
    pub fn right(self, board: Board) -> Piece {
        let mut new = self;
        new.col += 1;
        let max_col = PIECE_MAX_COLS[self.shape as usize][self.orientation as usize];

        if (new.col > max_col) || new.collides_in(board) {
            self
        } else {
            new
        }
    }

    /// Shift a piece down.  If impossible, returns the piece unchanged.
    #[must_use]
    pub fn down(self, board: Board) -> Piece {
        let mut new = self;
        new.row -= 1;

        if (new.row < 0) || new.collides_in(board) {
            self
        } else {
            new
        }
    }

    /// Check if a piece is valid (see [here](Piece#valid-pieces)).
    pub fn in_bounds(self) -> bool {
        let max_col = PIECE_MAX_COLS[self.shape as usize][self.orientation as usize];

        (self.col >= 0) && (self.col <= max_col) && (self.row >= 0) && (self.row <= 5)
    }

    /// Rotate a piece clockwise according to SRS.  If impossible, returns the
    /// piece unchanged.
    ///
    /// See [here](Piece#rotation-system) for more details.
    #[must_use]
    pub fn cw(self, board: Board) -> Piece {
        let orientation = self.orientation.cw();

        let kicks = &KICKS[self.shape as usize][self.orientation as usize];
        for (kick_col, kick_row) in kicks {
            let new = Piece {
                shape: self.shape,
                col: self.col + kick_col,
                row: self.row + kick_row,
                orientation,
            };

            if new.in_bounds() && !new.collides_in(board) {
                return new;
            }
        }

        self
    }

    /// Rotate a piece counter-clockwise according to SRS.  If impossible,
    /// returns the piece unchanged.
    ///
    /// See [here](Piece#rotation-system) for more details.
    #[must_use]
    pub fn ccw(self, board: Board) -> Piece {
        let orientation = self.orientation.ccw();

        let kicks = &KICKS[self.shape as usize][orientation as usize];
        for (kick_col, kick_row) in kicks {
            let new = Piece {
                shape: self.shape,
                col: self.col - kick_col,
                row: self.row - kick_row,
                orientation,
            };

            if new.in_bounds() && !new.collides_in(board) {
                return new;
            }
        }

        self
    }
}

/// The shape of each piece for each orientation, as a bit board.
///
/// Indexed first by piece [shape], then by [orientation].
///
/// These shapes always touch the bottom and left of the board, but not
/// necessarily the bottom-left corner.  For example, the Z piece in the default
/// orientation doesn't have a bottom-left corner (bit 0 is unset).
///
/// [shape]:       Shape
/// [orientation]: Orientation
pub static PIECE_SHAPES: [[u64; 4]; 7] = [
    [
        // I
        0b1111,
        0b1000000000100000000010000000001,
        0b1111,
        0b1000000000100000000010000000001,
    ],
    [
        // J
        0b10000000111,
        0b1100000000010000000001,
        0b1110000000100,
        0b1000000000100000000011,
    ],
    [
        // L
        0b1000000000111,
        0b100000000010000000011,
        0b1110000000001,
        0b1100000000100000000010,
    ],
    [
        // O
        0b110000000011,
        0b110000000011,
        0b110000000011,
        0b110000000011,
    ],
    [
        // S
        0b1100000000011,
        0b100000000110000000010,
        0b1100000000011,
        0b100000000110000000010,
    ],
    [
        // T
        0b100000000111,
        0b100000000110000000001,
        0b1110000000010,
        0b1000000000110000000010,
    ],
    [
        // Z
        0b110000000110,
        0b1000000000110000000001,
        0b110000000110,
        0b1000000000110000000001,
    ],
];

/// The maximum allowed column for a piece of the given shape and orientation.
///
/// Indexed first by piece [shape], then by [orientation].
///
/// A piece at this column will touch the right edge of the board, but still be
/// in bounds.  If it were one column right, it would be out of bounds.
///
/// [shape]:       Shape
/// [orientation]: Orientation
static PIECE_MAX_COLS: [[i8; 4]; 7] = [
    [6, 9, 6, 9], /* I */
    [7, 8, 7, 8], /* J */
    [7, 8, 7, 8], /* L */
    [8, 8, 8, 8], /* O */
    [7, 8, 7, 8], /* S */
    [7, 8, 7, 8], /* T */
    [7, 8, 7, 8], /* Z */
];

/// Kick data for the J, L, S, T, and Z pieces.
///
/// Referenced by [`KICKS`].
///
/// These pieces have bounding boxes that are exactly the same shape, so it
/// makes sense that they have the same kick data.
static JLSTZ_KICKS: [[(i8, i8); 5]; 4] = [
    [(1, -1), (0, -1), (0, 0), (1, -3), (0, -3)],
    [(-1, 0), (0, 0), (0, -1), (-1, 2), (0, 2)],
    [(0, 0), (1, 0), (1, 1), (0, -2), (1, -2)],
    [(0, 1), (-1, 1), (-1, 0), (0, 3), (-1, 3)],
];

/// Kick data for the I piece.
///
/// Referenced by [`KICKS`].
static I_KICKS: [[(i8, i8); 5]; 4] = [
    [(2, -2), (0, -2), (3, -2), (0, -3), (3, 0)],
    [(-2, 1), (-3, 1), (0, 1), (-3, 3), (0, 0)],
    [(1, -1), (3, -1), (0, -1), (3, 0), (0, -3)],
    [(-1, 2), (0, 2), (-3, 2), (0, 0), (-3, 3)],
];

/// Kick data for the O piece.
///
/// Referenced by [`KICKS`].
///
/// The O piece has 90-degree rotational symmetry, so it cannot kick.  In fact,
/// it can always rotate in place.  To match the shape of kick data, it's just a
/// bunch of zeros.
static O_KICKS: [[(i8, i8); 5]; 4] = [[(0, 0); 5]; 4];

/// Kick data for every piece, in every orientation.
///
/// References [`I_KICKS`], [`JLSTZ_KICKS`], and [`O_KICKS`].
///
/// Indexed first by piece [shape], then by [orientation], then finally by kick
/// number.
///
/// To rotate a piece clockwise, index by shape and **starting** orientation,
/// then **add** each kick `(column, row)` to the position and see if the piece
/// fits.
///
/// To rotate a piece counter-clockwise, index by shape and **final**
/// orientation, then **subtract** each kick `(column, row)` from the position
/// and see if the piece fits.
///
/// [shape]:       Shape
/// [orientation]: Orientation
static KICKS: [&[[(i8, i8); 5]; 4]; 7] = [
    &I_KICKS,     /* I */
    &JLSTZ_KICKS, /* J */
    &JLSTZ_KICKS, /* L */
    &O_KICKS,     /* O */
    &JLSTZ_KICKS, /* S */
    &JLSTZ_KICKS, /* T */
    &JLSTZ_KICKS, /* Z */
];

/// Bit mask for the bottom four rows (bottom 40 bits) of the game [board].
///
/// [board]: Board
const BOARD_MASK: u64 = 0b1111111111_1111111111_1111111111_1111111111;

impl Shape {
    /// Select a single bit according to a shape.
    ///
    /// There are 7 shapes, so every shape can fit in 8 bits.
    pub fn bit_mask(self) -> u8 {
        1 << (self as usize)
    }

    /// Array of all shapes.
    pub const ALL: [Shape; 7] = [
        Shape::I,
        Shape::J,
        Shape::L,
        Shape::O,
        Shape::S,
        Shape::T,
        Shape::Z,
    ];

    /// Get the single-character name of a shape.
    pub fn name(self) -> &'static str {
        ["I", "J", "L", "O", "S", "T", "Z"][self as usize]
    }

    /// Try to convert back from a `u8`.
    pub fn try_from(n: u8) -> Option<Shape> {
        match n {
            0 => Some(Shape::I),
            1 => Some(Shape::J),
            2 => Some(Shape::L),
            3 => Some(Shape::O),
            4 => Some(Shape::S),
            5 => Some(Shape::T),
            6 => Some(Shape::Z),
            _ => None,
        }
    }
}

impl Orientation {
    /// The orientation clockwise from the given one.
    pub fn cw(self) -> Orientation {
        use Orientation::*;
        match self {
            North => East,
            East => South,
            South => West,
            West => North,
        }
    }

    /// The orientation counter-clockwise from the given one.
    pub fn ccw(self) -> Orientation {
        use Orientation::*;
        match self {
            North => West,
            East => North,
            South => East,
            West => South,
        }
    }

    /// The orientation one half rotation from the given one.
    pub fn half(self) -> Orientation {
        use Orientation::*;
        match self {
            North => South,
            East => West,
            South => North,
            West => East,
        }
    }

    /// A canonical orientation for the given shape, with respect to symmetry.
    pub fn canonical(self, shape: Shape) -> Orientation {
        use Orientation::*;

        const SYM_90: [Orientation; 4] = [North, North, North, North];
        const SYM_180: [Orientation; 4] = [North, East, North, East];
        const SYM_360: [Orientation; 4] = [North, East, South, West];

        static CANONICAL: [[Orientation; 4]; 7] = [
            SYM_180, // I
            SYM_360, // J
            SYM_360, // L
            SYM_90,  // O
            SYM_180, // S
            SYM_360, // T
            SYM_180, // Z
        ];

        CANONICAL[shape as usize][self as usize]
    }

    /// Try to convert back from a `u8`.
    pub fn try_from(n: u8) -> Option<Orientation> {
        use Orientation::*;
        match n {
            0 => Some(North),
            1 => Some(East),
            2 => Some(South),
            3 => Some(West),
            _ => None,
        }
    }
}
