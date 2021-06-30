pub mod boardset;
pub mod gameplay;
pub mod gamestategraph;

use std::io::{stdout, Write};

use crate::gamestategraph::{Bag, GameStateGraph, QuantumBag};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    let mut graphs = Vec::new();
    graphs.push(GameStateGraph::new(QuantumBag::new(Bag::full())));

    for iter in 1..=10 {
        graphs.push(graphs.last().unwrap().step());

        writeln!(
            stdout,
            "After iteration {}, have {} boards ({} bags).",
            iter,
            graphs.last().unwrap().count(),
            graphs.last().unwrap().count_bags(),
        )?;
    }

    Ok(())
}
