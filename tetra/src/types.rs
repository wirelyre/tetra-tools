//! Types exported to Python.

use std::collections::HashSet;

use bitvec::prelude::*;
use pyo3::prelude::*;
use smallvec::SmallVec;
use strum::{EnumString, IntoStaticStr};

// Most methods are implemented in these submodules.
mod field;
mod fumen;
mod piece;
mod solution;

pub use srs_4l::gameplay::Physics;

/// Shape of a piece.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, EnumString, Eq, Hash, IntoStaticStr, PartialEq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Shape { I, J, L, O, S, T, Z }

/// Orientation of a piece on a board.
#[rustfmt::skip]
#[derive(Clone, Copy, Debug, EnumString, Eq, Hash, IntoStaticStr, PartialEq, PartialOrd, Ord)]
pub enum Orientation { North, East, South, West }

/// Piece in a solution, possibly broken across nonadjacent rows.
///
/// Immutable.  Values are validated at construction time.
#[pyclass(frozen)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Piece {
    #[pyo3(get)]
    pub shape: Shape,
    #[pyo3(get)]
    pub orientation: Orientation,
    #[pyo3(get)]
    pub column: u8,
    pub rows: BitArray<[u32; 1]>,
}

/// Resizable rectangular field of cells, each either empty or filled.  The
/// width is statically fixed, but the height can grow.
#[pyclass]
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Field(pub BitVec);

#[pyclass]
#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Solution {
    #[pyo3(get)]
    pub initial_field: Field,
    #[pyo3(get)]
    pub pieces: Vec<Piece>,
    #[pyo3(get)]
    pub held: Option<Shape>,
}

#[pyclass]
pub struct Fumen(pub ::fumen::Fumen);

#[pyclass]
pub struct QueueSet(pub HashSet<SmallVec<[Shape; 16]>>);

impl TryFrom<char> for Shape {
    type Error = ();

    fn try_from(value: char) -> Result<Self, ()> {
        match value {
            'I' => Ok(Shape::I),
            'J' => Ok(Shape::J),
            'L' => Ok(Shape::L),
            'O' => Ok(Shape::O),
            'S' => Ok(Shape::S),
            'T' => Ok(Shape::T),
            'Z' => Ok(Shape::Z),
            _ => Err(()),
        }
    }
}
