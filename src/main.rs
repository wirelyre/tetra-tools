pub mod gameplay;

use crossterm::{
    cursor,
    event::{read, Event, KeyCode, KeyEvent},
    queue,
    style::{PrintStyledContent, Stylize},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::io::{stdout, Write};

use gameplay::{Board, Piece, Shape};

fn main() -> std::io::Result<()> {
    let mut board = Board::empty();
    let mut piece = Piece::new(Shape::I);

    let mut stdout = stdout();
    queue!(stdout, EnterAlternateScreen)?;
    queue!(stdout, cursor::Hide)?;
    enable_raw_mode()?;

    loop {
        queue!(stdout, Clear(ClearType::All))?;
        queue!(stdout, cursor::MoveTo(0, 0))?;
        print_board(&mut stdout, board, Some(piece))?;
        stdout.flush()?;

        match read()? {
            Event::Key(KeyEvent { code, .. }) => match code {
                KeyCode::Left => piece = piece.left(board),
                KeyCode::Right => piece = piece.right(board),
                KeyCode::Down => piece = piece.down(board),

                KeyCode::Char('x') => piece = piece.ccw(board),
                KeyCode::Char('c') => piece = piece.cw(board),

                KeyCode::Char(' ') => {
                    if piece.can_place(board) {
                        board = piece.place(board);
                        piece = Piece::new(Shape::I);
                    }
                }

                KeyCode::Char('i') => piece = Piece::new(Shape::I),
                KeyCode::Char('j') => piece = Piece::new(Shape::J),
                KeyCode::Char('l') => piece = Piece::new(Shape::L),
                KeyCode::Char('o') => piece = Piece::new(Shape::O),
                KeyCode::Char('s') => piece = Piece::new(Shape::S),
                KeyCode::Char('t') => piece = Piece::new(Shape::T),
                KeyCode::Char('z') => piece = Piece::new(Shape::Z),

                KeyCode::Char('q') => break,

                _ => continue,
            },

            _ => continue,
        }
    }

    disable_raw_mode()?;
    queue!(stdout, cursor::Show)?;
    queue!(stdout, LeaveAlternateScreen)?;
    Ok(())
}

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
