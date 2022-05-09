use std::{fs::OpenOptions, io::BufWriter};

pub mod boardgraph;

fn main() -> std::io::Result<()> {
    let boards = boardgraph::compute();

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("legal-boards.leb128")?;

    let writer = BufWriter::new(file);

    srs_4l::board_list::write(&boards, writer)?;

    Ok(())
}
