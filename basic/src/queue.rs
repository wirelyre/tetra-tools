use std::collections::BTreeSet;

use crate::gameplay::Shape;

/// A sequence of up to 10 pieces.  The integer inside can be used to refer to
/// this queue by number.  However, it should mostly be treated as opaque data.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Queue(pub u32);

impl Queue {
    /// An empty queue.
    pub fn empty() -> Queue {
        Queue(0)
    }

    /// Push a shape onto the front of this queue.  The given shape will now be
    /// first.
    #[must_use]
    pub fn push_first(self, shape: Shape) -> Queue {
        let new = (shape as u32) + 1;
        let rest = self.0 << 3;
        Queue(new | rest)
    }

    /// Push a shape as the second into this queue.  The given shape will now be
    /// second.
    #[must_use]
    pub fn push_second(self, shape: Shape) -> Queue {
        let first = self.0 & 0b111;
        let new = ((shape as u32) + 1) << 3;
        let rest = (self.0 & !0b111) << 3;
        Queue(first | new | rest)
    }

    /// Push a shape onto the end of this queue.  The given shape will now be
    /// last.
    #[must_use]
    pub fn push_last(self, shape: Shape) -> Queue {
        let highest_one = 32 - self.0.leading_zeros();
        let rounded_up = (highest_one + 2) / 3 * 3;
        let new = ((shape as u32) + 1) << rounded_up;

        Queue(self.0 | new)
    }

    /// Produce a [`String`] containing the names of the shapes in this queue.
    pub fn to_string(self) -> String {
        let mut s = String::with_capacity(10);
        s.extend(self.map(Shape::name));
        s
    }

    /// Compute all queues which can be transformed into this queue using hold.
    ///
    /// This method assumes that the shapes in the provided queue are intended
    /// to be used exactly in order, without holding.  The returned queues are
    /// all queues which can be used *as though they were the provided queue* by
    /// using holding.

    pub fn unhold(self) -> BTreeSet<Queue> {
        let mut last = BTreeSet::new();

        let mut me: Vec<Shape> = self.collect();

        if let Some(shape) = me.pop() {
            last.insert(Queue::empty().push_first(shape));
        } else {
            last.insert(Queue::empty());
        }

        for &shape in me.iter().rev() {
            let mut next = BTreeSet::new();

            for queue in last {
                next.insert(queue.push_first(shape));
                next.insert(queue.push_second(shape));
            }

            last = next;
        }

        last
    }
}

impl Iterator for Queue {
    type Item = Shape;

    fn next(&mut self) -> Option<Shape> {
        let first = match self.0 & 0b111 {
            0 => None,
            1 => Some(Shape::I),
            2 => Some(Shape::J),
            3 => Some(Shape::L),
            4 => Some(Shape::O),
            5 => Some(Shape::S),
            6 => Some(Shape::T),
            7 => Some(Shape::Z),
            _ => unreachable!(),
        };

        self.0 = self.0 >> 3;

        first
    }
}
