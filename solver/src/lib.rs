use queue::Bag;
use std::{collections::HashSet, io::Cursor};
use wasm_bindgen::prelude::wasm_bindgen;

use basic::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Shape},
};

pub mod queue;
pub mod solver;

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

    pub fn solve(&self, queue: Queue, garbage: u64, can_hold: bool) -> String {
        let start = BrokenBoard::from_garbage(garbage);
        let solutions = solver::compute(&self.boards, &start, &queue.bags, can_hold);
        let mut str = String::new();

        for board in &solutions {
            solver::print(&board, &mut str);
            str.push(',');
        }

        str.pop();
        str
    }

    pub fn possible(&self, garbage: u64) -> bool {
        self.boards
            .contains(&BrokenBoard::from_garbage(garbage).board)
    }
}

#[wasm_bindgen]
pub struct Queue {
    bags: Vec<Bag>,
}

#[wasm_bindgen]
impl Queue {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Queue {
        Queue { bags: Vec::new() }
    }

    pub fn add_shape(&mut self, shape: char) {
        self.add_bag(&shape.to_string(), 1);
    }

    pub fn add_bag(&mut self, shapes: &str, count: u8) {
        let shapes = shapes
            .chars()
            .map(parse_shape)
            .collect::<Option<Vec<Shape>>>()
            .unwrap();
        self.bags.push(Bag::new(&shapes, count));
    }
}

fn parse_shape(shape: char) -> Option<Shape> {
    match shape {
        'I' => Some(Shape::I),
        'J' => Some(Shape::J),
        'L' => Some(Shape::L),
        'O' => Some(Shape::O),
        'S' => Some(Shape::S),
        'T' => Some(Shape::T),
        'Z' => Some(Shape::Z),
        _ => None,
    }
}

#[wasm_bindgen]
extern "C" {
    pub fn progress(piece_count: usize, stage: usize, board_idx: usize, board_total: usize);
}