use std::collections::HashSet;
use std::io::Cursor;

use gameplay::Board;

use crate::gameplay::Shape;

pub mod boardgraph;
pub mod brokenboard;
pub mod counter;
pub mod gameplay;

fn main() -> std::io::Result<()> {
    let contents = include_bytes!("../simple-boards.leb128");

    let legal_boards: HashSet<Board> = boardgraph::simple::read(Cursor::new(contents))?
        .drain(..)
        .collect();

    boardgraph::broken::compute(
        &legal_boards,
        &[
            Shape::T,
            Shape::S,
            Shape::Z,
            Shape::L,
            Shape::O,
            Shape::J,
            Shape::I,
            Shape::T,
            Shape::S,
            Shape::O,
        ],
    );

    Ok(())
}
