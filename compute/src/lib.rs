//! Useful data structures for computation, especially using multiple cores.

mod counter;
mod sharded_hashmap;

pub use counter::Counter;
pub use sharded_hashmap::*;
