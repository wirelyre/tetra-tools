//! Single-threaded graph implementation using broken boards.

use std::collections::{HashMap, HashSet};

use smallvec::SmallVec;

use crate::{
    boardgraph::PiecePlacer,
    brokenboard::BrokenBoard,
    gameplay::{Board, Shape},
};

type ScanStage = HashMap<Board, SmallVec<[Board; 6]>>;

fn scan(legal_boards: &HashSet<Board>, shapes: &[Shape]) -> Vec<ScanStage> {
    let mut stages = Vec::new();
    let mut last: ScanStage = HashMap::new();
    last.insert(Board::empty(), SmallVec::new());

    for &shape in shapes {
        let mut next: ScanStage = HashMap::new();

        for &old_board in last.keys() {
            for (_, new_board) in PiecePlacer::new(old_board, shape) {
                if !legal_boards.contains(&new_board) {
                    continue;
                }

                let preds = next.entry(new_board).or_default();
                if !preds.contains(&old_board) {
                    preds.push(old_board);
                }
            }
        }

        stages.push(last);
        last = next;
    }

    stages.push(last);

    stages
}

fn cull(scanned: &[ScanStage]) -> HashSet<Board> {
    let mut culled = HashSet::new();
    let mut iter = scanned.iter().rev();

    if let Some(final_stage) = iter.next() {
        for (&board, preds) in final_stage.iter() {
            culled.insert(board);
            culled.extend(preds);
        }
    }

    for stage in iter {
        for (&board, preds) in stage.iter() {
            if culled.contains(&board) {
                culled.extend(preds);
            }
        }
    }

    culled
}

fn place(culled: &HashSet<Board>, shapes: &[Shape]) -> HashSet<BrokenBoard> {
    let mut last = HashSet::new();
    last.insert(BrokenBoard::empty());

    for &shape in shapes {
        let mut next = HashSet::new();

        for board in last.iter() {
            for (piece, new_board) in PiecePlacer::new(board.board, shape) {
                if culled.contains(&new_board) {
                    next.insert(board.place(piece));
                }
            }
        }

        last = next;
    }

    last
}

pub fn compute(legal_boards: &HashSet<Board>, shapes: &[Shape]) {
    let scanned = scan(legal_boards, shapes);
    let culled = cull(&scanned);
    let mut placed = place(&culled, shapes);

    let mut solutions: Vec<BrokenBoard> = placed.drain().collect();
    solutions.sort_unstable();

    for solution in &solutions {
        println!();
        print(solution);
    }
}

pub fn print(board: &BrokenBoard) {
    let pieces: Vec<(Shape, Board)> = board
        .pieces
        .iter()
        .map(|&piece| (piece.shape, piece.board()))
        .collect();

    for row in (0..4).rev() {
        'cell: for col in 0..10 {
            for &(shape, board) in &pieces {
                if board.get(row, col) {
                    print!("{}", shape.name());
                    continue 'cell;
                }
            }
            print!("_");
        }
        println!();
    }
}
