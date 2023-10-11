use bitvec::prelude::*;
use pyo3::{exceptions::PyValueError, prelude::*};

use crate::types::{Field, Orientation, Piece, Shape};

impl From<Shape> for srs_4l::gameplay::Shape {
    fn from(value: Shape) -> Self {
        use srs_4l::gameplay::Shape as S;
        match value {
            Shape::I => S::I,
            Shape::J => S::J,
            Shape::L => S::L,
            Shape::O => S::O,
            Shape::S => S::S,
            Shape::T => S::T,
            Shape::Z => S::Z,
        }
    }
}

impl From<srs_4l::gameplay::Shape> for Shape {
    fn from(value: srs_4l::gameplay::Shape) -> Self {
        use srs_4l::gameplay::Shape as S;
        match value {
            S::I => Shape::I,
            S::J => Shape::J,
            S::L => Shape::L,
            S::O => Shape::O,
            S::S => Shape::S,
            S::T => Shape::T,
            S::Z => Shape::Z,
        }
    }
}

impl From<srs_4l::gameplay::Orientation> for Orientation {
    fn from(value: srs_4l::gameplay::Orientation) -> Self {
        use srs_4l::gameplay::Orientation as O;
        match value {
            O::North => Orientation::North,
            O::East => Orientation::East,
            O::South => Orientation::South,
            O::West => Orientation::West,
        }
    }
}

impl From<Orientation> for srs_4l::gameplay::Orientation {
    fn from(value: Orientation) -> Self {
        use srs_4l::gameplay::Orientation as O;
        match value {
            Orientation::North => O::North,
            Orientation::East => O::East,
            Orientation::South => O::South,
            Orientation::West => O::West,
        }
    }
}

impl From<srs_4l::brokenboard::BrokenPiece> for Piece {
    fn from(value: srs_4l::brokenboard::BrokenPiece) -> Self {
        use srs_4l::gameplay::Orientation as O;
        use srs_4l::gameplay::Shape as S;
        let offset = match (value.shape, value.orientation) {
            (S::J, O::South) => 2,
            (S::L, O::West) => 1,
            (S::S, O::East | O::West) => 1,
            (S::T, O::South | O::West) => 1,
            (S::Z, O::North | O::South) => 1,
            _ => 0,
        };

        Piece {
            shape: value.shape.into(),
            orientation: value.orientation.into(),
            column: value.low_mino % 10 - offset,
            rows: BitArray::new([value.rows as u32]),
        }
    }
}

impl From<&srs_4l::gameplay::Board> for Field {
    fn from(value: &srs_4l::gameplay::Board) -> Self {
        let mut f = bitvec![0; 40];
        f.clone_from_bitslice(&BitSlice::<u64, Lsb0>::from_element(&value.0)[..40]);
        Field(f)
    }
}

impl From<srs_4l::gameplay::Board> for Field {
    fn from(value: srs_4l::gameplay::Board) -> Self {
        value.into()
    }
}

impl TryFrom<&Field> for srs_4l::gameplay::Board {
    type Error = PyErr;

    fn try_from(value: &Field) -> Result<Self, Self::Error> {
        // TODO: Are taller fields okay if enough rows are cleared?
        if value.get_height() > 4 {
            return Err(PyValueError::new_err("field too tall"));
        }
        let mut b = srs_4l::gameplay::Board(0);
        BitSlice::<u64, Lsb0>::from_element_mut(&mut b.0)[..value.0.len()]
            .clone_from_bitslice(&value.0);
        Ok(b)
    }
}
