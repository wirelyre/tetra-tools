use js_sys::Uint8Array;
use miniserde::{json, Serialize};
use queue::Bag;
use std::{collections::HashSet, io::Cursor};
use wasm_bindgen::prelude::wasm_bindgen;

use srs_4l::{
    base64::{base64_decode, base64_encode},
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
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
    pub fn init(legal_boards: Option<Uint8Array>) -> Solver {
        let boards: HashSet<Board> = match legal_boards {
            Some(arr) => board_list::read(Cursor::new(&arr.to_vec()))
                .unwrap()
                .drain(..)
                .collect(),
            None => Default::default(),
        };

        Solver { boards }
    }

    pub fn solve(&self, queue: Queue, garbage: u64, can_hold: bool, physics: String) -> String {
        let empty_boards = Default::default();

        let start = BrokenBoard::from_garbage(garbage);

        let legal_boards = if self.is_fast(garbage) {
            &self.boards
        } else {
            &empty_boards
        };

        let physics = match physics.as_ref() {
            "SRS" => Physics::SRS,
            "Jstris" => Physics::Jstris,
            "TETRIO" => Physics::Tetrio,
            _ => return "".into(),
        };

        let solutions = solver::compute(legal_boards, &start, &queue.bags, can_hold, physics);
        let mut str = String::new();

        for board in &solutions {
            solver::print(&board, &mut str);
            str.push('|');
            base64_encode(&board.encode(), &mut str);
            str.push(',');
        }

        str.pop();
        str
    }

    pub fn is_fast(&self, garbage: u64) -> bool {
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

#[wasm_bindgen]
pub fn solution_info(encoded: &str) -> String {
    let mut ret = "".to_string();

    let bits = match base64_decode(encoded) {
        Some(b) => b,
        None => return ret,
    };

    let board = match BrokenBoard::decode(&bits) {
        Some(b) => b,
        None => return ret,
    };

    // TODO:  Return queues classified by physics.
    let mut without_hold = board.supporting_queues(Physics::SRS);
    without_hold.sort_unstable_by_key(|q| q.natural_order_key());

    let with_hold = srs_4l::queue::Queue::unhold_many(&without_hold);

    solver::print(&board, &mut ret);

    ret.push('|');

    for &queue in &without_hold {
        ret.push_str(&queue.to_string());
        ret.push(',');
    }
    if !without_hold.is_empty() {
        ret.pop();
    }

    ret.push('|');

    for &queue in &with_hold {
        ret.push_str(&queue.to_string());
        ret.push(',');
    }
    if !with_hold.is_empty() {
        ret.pop();
    }

    ret
}

#[wasm_bindgen]
pub fn decode_fumen(encoded: &str) -> String {
    #[derive(Default, Serialize)]
    struct Decoded {
        field: u64,
        comment: Option<String>,
    }

    fn inner(encoded: &str) -> Option<Decoded> {
        use fumen::{CellColor, Fumen, Page};

        let fumen = Fumen::decode(encoded).ok()?;
        let page: &Page = fumen.pages.get(0)?;

        if page.field[4..] != [[CellColor::Empty; 10]; 19]
            || page.garbage_row != [CellColor::Empty; 10]
        {
            return None;
        }

        let mut field = 0;
        for idx in 0..40 {
            let cell: CellColor = page.field[idx / 10][idx % 10];
            let filled = cell != CellColor::Empty;
            field |= (filled as u64) << idx;
        }

        let comment = page.comment.clone();
        Some(Decoded { field, comment })
    }

    json::to_string(&inner(encoded))
}
