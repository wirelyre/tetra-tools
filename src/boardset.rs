use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;

use crate::gameplay::Board;

pub struct BoardSet(pub Vec<Arc<Mutex<HashSet<u64>>>>);

const LOW_BITS_MASK: u64 = 0b1111111111;
// const LOW_BITS_MASK: u64 = 0b1111111111_1111111111;

impl BoardSet {
    pub fn new() -> Self {
        let mut v = Vec::new();

        for _ in 0..(LOW_BITS_MASK + 1) {
            v.push(Arc::new(Mutex::new(HashSet::new())))
        }

        BoardSet(v)
    }

    pub fn get(&self, board: Board) -> bool {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let subset = self.0[low_bits].lock();
        subset.contains(&board.0)
    }

    pub fn insert(&self, board: Board) {
        let low_bits = (board.0 & LOW_BITS_MASK) as usize;
        let mut subset = self.0[low_bits].lock();
        subset.insert(board.0);
    }
}
