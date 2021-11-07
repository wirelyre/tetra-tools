use std::fs::OpenOptions;
use std::io::BufWriter;

pub mod boardgraph;
pub mod counter;

fn main() -> std::io::Result<()> {
    let boards = boardgraph::simple::compute();

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("simple-boards.leb128")?;

    let writer = BufWriter::new(file);

    basic::board_list::write(&boards, writer)?;

    Ok(())
}
