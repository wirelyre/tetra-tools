use crate::types::Shape;

impl From<fumen::PieceType> for Shape {
    fn from(value: fumen::PieceType) -> Self {
        use fumen::PieceType as P;
        use Shape as S;
        match value {
            P::I => S::I,
            P::J => S::J,
            P::L => S::L,
            P::O => S::O,
            P::S => S::S,
            P::T => S::T,
            P::Z => S::Z,
        }
    }
}

impl From<Shape> for fumen::PieceType {
    fn from(value: Shape) -> Self {
        use fumen::PieceType as P;
        use Shape as S;
        match value {
            S::I => P::I,
            S::J => P::J,
            S::L => P::L,
            S::O => P::O,
            S::S => P::S,
            S::T => P::T,
            S::Z => P::Z,
        }
    }
}

impl From<Shape> for fumen::CellColor {
    fn from(value: Shape) -> Self {
        fumen::PieceType::from(value).into()
    }
}
