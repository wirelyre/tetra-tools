pub mod boardset;
pub mod gameplay;

use bitvec::prelude::*;
use crossterm::{
    cursor, queue,
    style::{PrintStyledContent, Stylize},
};
use rayon::prelude::*;
use std::io::{stdout, Write};

use boardset::BoardSet;
use gameplay::{Board, Piece, Shape};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    let mut results = BoardSet::new();
    results.insert(Board::empty());

    for iter in /* 1..=10 */ 1..=5 {
        results = results.par_iter().flat_map_iter(process_board).collect();

        let mut count = 0;
        for set in &results.0 {
            count += set.lock().len();
        }

        writeln!(stdout, "After iteration {}, have {} boards.", iter, count)?;
    }

    Ok(())
}

fn process_board(board: Board) -> Vec<Board> {
    let mut queue = vec![
        Piece::new(Shape::I),
        Piece::new(Shape::J),
        Piece::new(Shape::L),
        Piece::new(Shape::O),
        Piece::new(Shape::S),
        Piece::new(Shape::T),
        Piece::new(Shape::Z),
    ];
    let mut seen = bitvec![0; 0x4000];
    let mut out = Vec::new();

    for &piece in &queue {
        seen.set(piece.pack() as usize, true);
    }

    while let Some(piece) = queue.pop() {
        for &new_piece in &[
            piece.left(board),
            piece.right(board),
            piece.down(board),
            piece.cw(board),
            piece.ccw(board),
        ] {
            if !seen[new_piece.pack() as usize] {
                seen.set(new_piece.pack() as usize, true);

                queue.push(new_piece);

                if new_piece.can_place(board) {
                    out.push(new_piece.place(board));
                }
            }
        }
    }

    out
}

#[allow(dead_code)]
fn print_board(out: &mut impl Write, board: Board, piece: Option<Piece>) -> std::io::Result<()> {
    let piece_board = piece.map(Piece::as_board);

    for row in (0..=3).rev() {
        for col in 0..=9 {
            if piece_board.map(|p| p.get(row, col)).unwrap_or(false) {
                queue!(out, PrintStyledContent("█".black()))?;
            } else if board.get(row, col) {
                queue!(out, PrintStyledContent("█".grey()))?;
            } else {
                queue!(out, PrintStyledContent("█".white()))?;
            }
        }

        queue!(out, cursor::MoveToNextLine(1))?;
    }

    Ok(())
}
