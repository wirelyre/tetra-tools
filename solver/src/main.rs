use std::collections::HashSet;
use std::io::Cursor;

use basic::{
    board_list,
    gameplay::{Board, Shape},
};

mod broken;

fn main() -> std::io::Result<()> {
    let contents = include_bytes!("../../simple-boards.leb128");

    let legal_boards: HashSet<Board> = board_list::read(Cursor::new(contents))?.drain(..).collect();

    broken::compute(
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
