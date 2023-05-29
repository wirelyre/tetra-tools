pub mod parse;
pub mod physics;
pub mod placements;

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

#[repr(u8)]
pub enum Orientation {
    North,
    East,
    South,
    West,
}

#[repr(u8)]
pub enum Rotation {
    Clockwise,
    Half,
    CounterClockwise,
}
