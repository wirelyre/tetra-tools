use std::io::{self, Read, Write};

use crate::gameplay::Board;

pub fn write(boards: &[Board], mut w: impl Write) -> io::Result<()> {
    leb128::write::unsigned(&mut w, boards.len() as u64)?;

    let mut current = 0;

    for &board in boards {
        let diff = board.0 - current;
        current = board.0;

        leb128::write::unsigned(&mut w, diff)?;
    }

    Ok(())
}

pub fn read(mut r: impl Read) -> io::Result<Vec<Board>> {
    fn to_io_error(err: leb128::read::Error) -> io::Error {
        use leb128::read::Error;

        match err {
            Error::IoError(err) => err,
            Error::Overflow => io::Error::new(io::ErrorKind::InvalidData, err),
        }
    }

    let len = leb128::read::unsigned(&mut r).map_err(to_io_error)? as usize;

    let mut boards = Vec::new();
    let mut current = 0;

    for _ in 0..len {
        let diff = leb128::read::unsigned(&mut r).map_err(to_io_error)?;
        current += diff;
        boards.push(Board(current));
    }

    Ok(boards)
}
