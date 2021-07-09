use std::fs::OpenOptions;

pub mod boardgraph;
pub mod counter;
pub mod gameplay;

fn main() -> std::io::Result<()> {
    let boards = boardgraph::simple::compute();

    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open("simple-boards.zstd")?;

    let mut encoder = zstd::Encoder::new(file, 21)?;

    serde_json::to_writer(&mut encoder, &boards)?;

    encoder.finish()?;

    Ok(())
}
