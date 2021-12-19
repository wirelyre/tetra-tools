use std::{collections::HashSet, io::Cursor};
use wasm_bindgen::prelude::wasm_bindgen;

use basic::{
    board_list,
    brokenboard::BrokenBoard,
    gameplay::{Board, Shape},
    piece_placer::PiecePlacer,
};

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

    pub fn candidates(&self, board: u64) -> String {
        let board = Board(board);
        let mut boards = HashSet::new();

        for shape in Shape::ALL {
            for (_, new_board) in PiecePlacer::new(board, shape) {
                if new_board.has_isolated_cell() || new_board.has_imbalanced_split() {
                    continue;
                }

                if !self.boards.contains(&new_board) {
                    boards.insert(new_board);
                }
            }
        }

        let mut boards: Vec<_> = boards.drain().collect();
        boards.sort_unstable();

        let mut str = String::new();

        for board in boards {
            print_board(board, &mut str);
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

fn print_board(board: Board, into: &mut String) {
    for row in (0..4).into_iter().rev() {
        for col in 0..10 {
            if board.get(row, col) {
                into.push('G');
            } else {
                into.push('_');
            }
        }
    }
}
