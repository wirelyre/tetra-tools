use smallvec::SmallVec;

use srs_4l::gameplay::Shape;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Bag {
    pub count: u8,
    pub full: u16,
    pub masks: [u16; 7],
}

impl Bag {
    pub fn new(shapes: &[Shape], count: u8) -> Bag {
        assert!(count as usize <= shapes.len());
        assert!(shapes.len() <= 13);

        let mut bag = Bag {
            count,
            full: (1 << shapes.len()) - 1,
            masks: [0; 7],
        };

        for (i, &shape) in shapes.iter().enumerate() {
            bag.masks[shape as usize] |= 1 << i;
        }

        bag
    }

    pub fn init_hold(&self) -> SmallVec<[QueueState; 7]> {
        let initial = QueueState(self.full);

        Shape::ALL
            .iter()
            .filter_map(|&shape| initial.swap(self, shape))
            .collect()
    }

    pub fn take(
        &self,
        queues: &[QueueState],
        shape: Shape,
        is_first: bool,
        can_hold: bool,
    ) -> SmallVec<[QueueState; 7]> {
        let mut states = SmallVec::new();

        for &queue in queues {
            let queue = if is_first { queue.next(self) } else { queue };

            if queue.hold() == Some(shape) {
                for swap_shape in Shape::ALL {
                    if let Some(new) = queue.swap(self, swap_shape) {
                        if !states.contains(&new) {
                            states.push(new);
                        }
                    }
                }
            } else if can_hold {
                if let Some(new) = queue.take(self, shape) {
                    if !states.contains(&new) {
                        states.push(new);
                    }
                }
            }
        }

        states
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueueState(pub u16);

impl QueueState {
    pub fn hold(self) -> Option<Shape> {
        match self.0 >> 13 {
            0 => Some(Shape::I),
            1 => Some(Shape::J),
            2 => Some(Shape::L),
            3 => Some(Shape::O),
            4 => Some(Shape::S),
            5 => Some(Shape::T),
            6 => Some(Shape::Z),
            _ => None,
        }
    }

    pub fn next(self, bag: &Bag) -> QueueState {
        QueueState(self.0 & 0b1110000000000000 | bag.full)
    }

    pub fn take(self, bag: &Bag, shape: Shape) -> Option<QueueState> {
        let shape_field = self.0 & bag.masks[shape as usize];

        if shape_field == 0 {
            return None;
        }

        let new_shape_field = shape_field & (shape_field - 1);
        Some(QueueState(self.0 ^ shape_field ^ new_shape_field))
    }

    pub fn swap(self, bag: &Bag, shape: Shape) -> Option<QueueState> {
        let mut new = self.take(bag, shape)?;
        new.0 &= 0b1111111111111;
        new.0 |= (shape as u16) << 13;
        Some(new)
    }
}
