use crate::{
    physics::{Chunk, Physics},
    Orientation, Shape,
};

pub struct Placements<C: Chunk, const N: usize>(pub [[C; N]; 4]);

impl<C: Chunk, const N: usize> Placements<C, N> {
    pub fn len(&self) -> usize {
        self.0
            .iter()
            .flatten()
            .map(|c| c.count_set() as usize)
            .sum()
    }
}

impl<C: Chunk, const N: usize> Iterator for Placements<C, N> {
    type Item = (Shape, Orientation, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.iter().flatten().all(|c| c.is_empty()) {
            return None;
        }

        todo!()
    }
}

pub struct PlacementMachine<C: Chunk, const N: usize> {
    pub field: [C; N], // TODO: wrong length
    pub shape: Shape,
    pub dirty: u32,
    pub reachable: [[C; N]; 4],
    pub viable: [[C; N]; 4],
}

impl<C: Chunk, const N: usize> PlacementMachine<C, N> {
    pub fn kicks(&mut self, physics: &Physics, from: Orientation, chunk: usize, to: Orientation) {
        let _ = (physics, from, chunk, to);
        todo!()
    }
}
