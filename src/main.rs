pub mod boardgraph;
pub mod counter;
pub mod gameplay;

use boardgraph::simple::SimpleGraph;

fn main() {
    let _graph = SimpleGraph::compute();
}
