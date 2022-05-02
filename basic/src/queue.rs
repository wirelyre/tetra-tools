use std::{borrow::Borrow, collections::BTreeSet, iter::FromIterator};

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

    pub fn is_empty(self) -> bool {
        self.0 == 0
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
        assert!(!self.is_empty()); // otherwise this method doesn't make sense

        let first = self.0 & 0b111;
        let new = ((shape as u32) + 1) << 3;
        let rest = (self.0 & !0b111) << 3;
        Queue(first | new | rest)
    }

    /// Push a shape onto the end of this queue.  The given shape will now be
    /// last.
    #[must_use]
    pub fn push_last(self, shape: Shape) -> Queue {
        let next_slot = self.len() * 3;
        let new = ((shape as u32) + 1) << next_slot;

        Queue(self.0 | new)
    }

    pub fn len(self) -> u32 {
        let highest_one = 32 - self.0.leading_zeros();
        (highest_one + 2) / 3
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

    pub fn unhold_many(queues: &[Queue]) -> Vec<Queue> {
        let mut results: Vec<BTreeSet<Entry>> = Vec::new();
        results.resize_with(11, || BTreeSet::new());

        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        struct Entry {
            make: Queue,
            take: Queue,
        }

        for &queue in queues {
            results[queue.len() as usize].insert(Entry {
                make: Queue::empty(),
                take: queue.reverse(),
            });
        }

        for i in (1..=10).into_iter().rev() {
            let (next, this) = results.split_at_mut(i);
            let next = next.last_mut().unwrap();
            let this = this.first().unwrap();

            for entry in this {
                let mut take = entry.take;
                let shape = take.next().unwrap();

                next.insert(Entry {
                    make: entry.make.push_first(shape),
                    take,
                });

                if !entry.make.is_empty() {
                    next.insert(Entry {
                        make: entry.make.push_second(shape),
                        take,
                    });
                }
            }
        }

        let mut results: Vec<Queue> = results[0].iter().map(|e| e.make).collect();
        results.sort_unstable_by_key(|q| q.natural_order_key());
        results
    }

    pub fn natural_order_key(self) -> u32 {
        #![allow(non_snake_case)]

        let jihgfedcba = self.0;
        let hgfedcba = jihgfedcba & 0o77777777;

        let dcba____ = hgfedcba << 12 & 0o77770000;
        let ____hgfe = hgfedcba >> 12;
        let dcbahgfe = dcba____ | ____hgfe;

        let ba__fe__ = dcbahgfe << 6 & 0o77007700;
        let __dc__hg = dcbahgfe >> 6 & 0o00770077;
        let badcfehg = ba__fe__ | __dc__hg;

        let badcfehgji = badcfehg << 6 | jihgfedcba >> 24;

        let a_c_e_g_i_ = badcfehgji << 3 & 0o7070707070;
        let _b_d_f_h_j = badcfehgji >> 3 & 0o0707070707;
        let abcdefghij = a_c_e_g_i_ | _b_d_f_h_j;

        abcdefghij
    }

    #[must_use]
    pub fn reverse(self) -> Queue {
        let x = self.natural_order_key();
        Queue(x >> (x.trailing_zeros() / 3 * 3))
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

impl<S: Borrow<Shape>> Extend<S> for Queue {
    fn extend<T: IntoIterator<Item = S>>(&mut self, iter: T) {
        for shape in iter {
            if self.len() == 10 {
                break;
            }

            *self = self.push_last(*shape.borrow());
        }
    }
}

impl<S: Borrow<Shape>> FromIterator<S> for Queue {
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Queue {
        let mut queue = Queue::empty();
        queue.extend(iter);
        queue
    }
}

#[cfg(test)]
mod tests {
    use crate::{gameplay::Shape, queue::Queue};

    #[test]
    fn order() {
        use Shape::*;

        let mut queues: Vec<Queue> = vec![
            [I, I, I, I].iter().collect(),
            [J, I, L].iter().collect(),
            [I, I, L].iter().collect(),
            [I, J, L].iter().collect(),
            [J, J, J, I].iter().collect(),
            [T].iter().collect(),
        ];
        queues.sort_unstable_by_key(|q| q.natural_order_key());

        let expected: &[Queue] = &[
            [I, I, I, I].iter().collect(),
            [I, I, L].iter().collect(),
            [I, J, L].iter().collect(),
            [J, I, L].iter().collect(),
            [J, J, J, I].iter().collect(),
            [T].iter().collect(),
        ];

        assert_eq!(queues, expected);
    }

    #[test]
    fn reverse() {
        use Shape::*;

        fn reverse_eq(q1: &[Shape], q2: &[Shape]) {
            let q1: Queue = q1.iter().collect();
            let q2: Queue = q2.iter().collect();

            assert_eq!(q1.reverse(), q2);
            assert_eq!(q1, q2.reverse());
        }

        reverse_eq(&[], &[]);
        reverse_eq(&[I], &[I]);
        reverse_eq(&[I, J, L, O, S, T, Z, I, J], &[J, I, Z, T, S, O, L, J, I]);
        reverse_eq(
            &[I, J, L, O, S, T, Z, I, J, L],
            &[L, J, I, Z, T, S, O, L, J, I],
        );
    }

    #[test]
    #[rustfmt::skip]
    fn unhold_many() {
        use Shape::*;

        fn unhold_len(queues: &[&[Shape]], expected_len: usize) {
            let queues: Vec<Queue> = queues
                .iter()
                .map(|shapes| shapes.iter().collect())
                .collect();
            let unheld = Queue::unhold_many(&queues);

            assert_eq!(unheld.len(), expected_len);
        }

        // length-10 queues
        unhold_len(&[
                &[I, I, I, I, I, I, I, I, I, T], // from TII…, ITI…, etc.
                &[J, J, J, J, J, J, J, J, J, T], // from TJJ…, JTJ…, etc.
            ], 10 + 10);

        // shared unheld queues
        unhold_len(&[
                &[I, J, L], // from IJL, ILJ, JIL,      LIJ
                &[J, I, L], // from IJL,      JIL, JLI,     LJI
            ], 6);

        // mixed-length queues
        unhold_len(&[
                &[T],
                &[I, T],
                &[I, I, T],
                &[I, I, I, T],
            ], 1 + 2 + 3 + 4);
    }
}
