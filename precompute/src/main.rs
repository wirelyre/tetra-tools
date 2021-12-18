use std::fs::OpenOptions;
use std::io::{BufWriter, Write};

pub mod boardgraph;
pub mod counter;

fn main() -> std::io::Result<()> {
    let boards = boardgraph::start_anywhere::compute();

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("start-anywhere-boards.leb128.zstd")?;

    let writer = BufWriter::new(file);
    let mut writer = zstd::Encoder::new(writer, 21)?;
    eprintln!("writing board list...");
    basic::board_list::write(&boards, &mut writer)?;
    writer.finish()?.flush()?;
    eprintln!("done");

    Ok(())
}
