use std::fs::OpenOptions;
use std::io::BufWriter;

pub mod boardgraph;
pub mod counter;
pub mod gameplay;

fn main() -> std::io::Result<()> {
    let boards = boardgraph::simple::compute();

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("simple-boards.bin")?;

    let writer = BufWriter::new(file);

    boardgraph::simple::write(&boards, writer)?;

    Ok(())
}
