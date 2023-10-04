//! Single-threaded solver that produces broken boards.

use std::collections::{HashMap, HashSet};

use smallvec::SmallVec;

use srs_4l::{
    brokenboard::BrokenBoard,
    gameplay::{Board, Physics, Shape},
    vector::Placements,
};

use crate::queue::{Bag, QueueState};

type ScanStage = HashMap<Board, (SmallVec<[QueueState; 7]>, SmallVec<[Board; 6]>)>;

fn scan(
    legal_boards: &HashSet<Board>,
    start: Board,
    bags: &[Bag],
    piece_count: usize,
    can_hold: bool,
    place_last: bool,
    physics: Physics,
) -> Vec<ScanStage> {
    let mut stages = Vec::new();

    let mut prev: ScanStage = HashMap::new();
    prev.insert(start, (bags.first().unwrap().init_hold(), SmallVec::new()));

    for (stage, (bag, i)) in bags
        .iter()
        .flat_map(|b| (0..b.count).into_iter().map(move |i| (b, i)))
        .skip(1)
        .enumerate()
    {
        let mut next: ScanStage = HashMap::new();

        for (board_idx, (&old_board, (old_queues, _preds))) in prev.iter().enumerate() {
            if board_idx % 4096 == 0 {
                crate::progress(piece_count, stage, board_idx, prev.len());
            }

            for shape in Shape::ALL {
                let is_first = i == 0;
                let new_queues = bag.take(old_queues, shape, is_first, can_hold);

                if new_queues.is_empty() {
                    continue;
                }

                for (_, new_board) in Placements::place(old_board, shape, physics).canonical() {
                    if !legal_boards.is_empty() && !legal_boards.contains(&new_board) {
                        continue;
                    }

                    let (queues, preds) = next.entry(new_board).or_default();
                    if !preds.contains(&old_board) {
                        preds.push(old_board);
                    }
                    for &queue in &new_queues {
                        if !queues.contains(&queue) {
                            queues.push(queue);
                        }
                    }
                }
            }
        }

        stages.push(prev);
        prev = next;
    }

    if place_last {
        let mut next: ScanStage = HashMap::new();

        for (board_idx, (&old_board, (old_queues, _preds))) in prev.iter().enumerate() {
            if board_idx % 4096 == 0 {
                crate::progress(piece_count, piece_count, board_idx, prev.len());
            }

            for shape in Shape::ALL {
                if old_queues.iter().any(|queue| queue.hold() == Some(shape)) {
                    for (_, new_board) in Placements::place(old_board, shape, physics).canonical() {
                        if !legal_boards.is_empty() && !legal_boards.contains(&new_board) {
                            continue;
                        }

                        let (_queues, preds) = next.entry(new_board).or_default();
                        if !preds.contains(&old_board) {
                            preds.push(old_board);
                        }
                    }
                }
            }
        }

        stages.push(prev);
        prev = next;
    }

    crate::progress(piece_count, piece_count, 1, 1);

    stages.push(prev);
    stages
}

fn cull(scanned: &[ScanStage]) -> HashSet<Board> {
    let mut culled = HashSet::new();
    let mut iter = scanned.iter().rev();

    if let Some(final_stage) = iter.next() {
        for (&board, (_queues, preds)) in final_stage.iter() {
            culled.insert(board);
            culled.extend(preds);
        }
    }

    for stage in iter {
        for (&board, (_queues, preds)) in stage.iter() {
            if culled.contains(&board) {
                culled.extend(preds);
            }
        }
    }

    culled
}

fn place(
    culled: &HashSet<Board>,
    start: BrokenBoard,
    bags: &[Bag],
    piece_count: usize,
    can_hold: bool,
    place_last: bool,
    physics: Physics,
) -> HashMap<BrokenBoard, SmallVec<[QueueState; 7]>> {
    let mut prev = HashMap::new();
    prev.insert(start, bags.first().unwrap().init_hold());

    for (stage, (bag, i)) in bags
        .iter()
        .flat_map(|b| (0..b.count).into_iter().map(move |i| (b, i)))
        .skip(1)
        .enumerate()
    {
        let mut next: HashMap<BrokenBoard, SmallVec<[QueueState; 7]>> = HashMap::new();

        for (board_idx, (old_board, old_queues)) in prev.iter().enumerate() {
            if board_idx % 4096 == 0 {
                crate::progress(piece_count, piece_count + 1 + stage, board_idx, prev.len());
            }

            for shape in Shape::ALL {
                let is_first = i == 0;
                let new_queues = bag.take(old_queues, shape, is_first, can_hold);

                if new_queues.is_empty() {
                    continue;
                }

                for (piece, new_board) in
                    Placements::place(old_board.board, shape, physics).canonical()
                {
                    if culled.contains(&new_board) {
                        let queues = next.entry(old_board.place(piece)).or_default();
                        for &queue in &new_queues {
                            if !queues.contains(&queue) {
                                queues.push(queue);
                            }
                        }
                    }
                }
            }
        }

        prev = next;
    }

    if place_last {
        let mut next: HashMap<BrokenBoard, SmallVec<[QueueState; 7]>> = HashMap::new();

        for (board_idx, (old_board, old_queues)) in prev.iter().enumerate() {
            if board_idx % 4096 == 0 {
                crate::progress(piece_count, 2 * piece_count + 1, board_idx, prev.len());
            }

            for shape in Shape::ALL {
                if old_queues.iter().any(|queue| queue.hold() == Some(shape)) {
                    for (piece, new_board) in
                        Placements::place(old_board.board, shape, physics).canonical()
                    {
                        if culled.contains(&new_board) {
                            next.insert(old_board.place(piece), SmallVec::new());
                        }
                    }
                }
            }
        }

        prev = next;
    }

    crate::progress(piece_count, 2 * piece_count + 1, 1, 1);

    prev
}

pub fn compute(
    legal_boards: &HashSet<Board>,
    start: &BrokenBoard,
    bags: &[Bag],
    can_hold: bool,
    physics: Physics,
) -> Vec<BrokenBoard> {
    if bags.is_empty() {
        return vec![start.clone()];
    }

    let piece_count = bags.iter().map(|b| b.count as usize).sum();
    let new_mino_count = piece_count as u32 * 4;
    let place_last = start.board.0.count_ones() + new_mino_count <= 40;

    let scanned = scan(
        legal_boards,
        start.board,
        bags,
        piece_count,
        can_hold,
        place_last,
        physics,
    );
    let culled = cull(&scanned);
    let mut placed = place(
        &culled,
        start.clone(),
        bags,
        piece_count,
        can_hold,
        place_last,
        physics,
    );

    let mut solutions: Vec<BrokenBoard> =
        placed.drain().map(|(board, _queue_states)| board).collect();
    solutions.sort_unstable();

    solutions
}

pub fn print(board: &BrokenBoard, to: &mut String) {
    let pieces: Vec<(Shape, Board)> = board
        .pieces
        .iter()
        .map(|&piece| (piece.shape, piece.board()))
        .collect();
    let bits = board.to_broken_bitboard();

    for row in (0..4).rev() {
        'cell: for col in 0..10 {
            for &(shape, board) in &pieces {
                if board.get(row, col) {
                    to.push_str(shape.name());
                    continue 'cell;
                }
            }

            if bits.get(row, col) {
                to.push('G');
            } else {
                to.push('_');
            }
        }
    }
}
