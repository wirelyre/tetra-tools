pub mod boardgraph;
pub mod gameplay;

use std::io::{stdout, Write};

use crate::boardgraph::gamestate::{GameStateStage, QuantumBag};

fn main() -> std::io::Result<()> {
    let mut stdout = stdout();

    let mut stages = Vec::new();
    stages.push(GameStateStage::new(QuantumBag::every_bag_no_hold()));

    for iter in 1..=4 {
        stages.push(stages.last().unwrap().step());

        writeln!(
            stdout,
            "After iteration {}, have {} boards ({} bags).",
            iter,
            stages.last().unwrap().count_boards(),
            stages.last().unwrap().count_bags(),
        )?;
    }

    Ok(())
}
