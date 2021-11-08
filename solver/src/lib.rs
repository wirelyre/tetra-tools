use std::{collections::HashSet, io::Cursor};
use wasm_bindgen::prelude::wasm_bindgen;

use basic::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Shape},
};

pub mod broken;

#[wasm_bindgen]
pub struct Solver {
    boards: HashSet<Board>,
}

#[wasm_bindgen]
impl Solver {
    #[wasm_bindgen(constructor)]
    pub fn init() -> Solver {
        let contents = include_bytes!("../../simple-boards.leb128");

        let boards: HashSet<Board> = board_list::read(Cursor::new(contents))
            .unwrap()
            .drain(..)
            .collect();

        Solver { boards }
    }

    pub fn solve_some(&self, pieces: &str, garbage: u64, count: usize) -> String {
        if let Some(shapes) = parse_shapes(pieces) {
            let start = BrokenBoard::from_garbage(garbage);
            let mut solutions = broken::compute(&self.boards, &start, &shapes);
            let mut str = format!("{}", solutions.len());

            solutions.truncate(count);

            for board in &solutions {
                str.push(',');
                broken::print(&board, start.board, &mut str);
            }

            str
        } else {
            String::new()
        }
    }

    pub fn solve(&self, pieces: &str, garbage: u64) -> String {
        if let Some(shapes) = parse_shapes(pieces) {
            let start = BrokenBoard::from_garbage(garbage);
            let solutions = broken::compute(&self.boards, &start, &shapes);
            let mut str = format!("{}", solutions.len());

            for board in &solutions {
                str.push(',');
                broken::print(&board, start.board, &mut str);
            }

            str
        } else {
            String::new()
        }
    }

    pub fn possible(&self, garbage: u64) -> bool {
        self.boards
            .contains(&BrokenBoard::from_garbage(garbage).board)
    }
}

fn parse_shapes(shapes: &str) -> Option<Vec<Shape>> {
    let mut vec = Vec::new();

    for shape in shapes.chars() {
        let shape = match shape {
            'I' => Shape::I,
            'J' => Shape::J,
            'L' => Shape::L,
            'O' => Shape::O,
            'S' => Shape::S,
            'T' => Shape::T,
            'Z' => Shape::Z,
            _ => return None,
        };

        vec.push(shape);
    }

    if vec.len() > 10 {
        return None;
    }

    Some(vec)
}
