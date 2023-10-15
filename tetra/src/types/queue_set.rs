use std::{collections::BTreeSet, sync::OnceLock};

use ahash::AHashSet;
use pyo3::{
    exceptions::{PyIndexError, PyValueError},
    prelude::*,
    types::PyString,
};
use rdst::RadixSort;
use regex::{Captures, Regex};
use tap::prelude::*;

use crate::types::{QueueSet, Shape};

/// Queue holding up to 20 pieces.
///
/// The queue pattern `*7*7*6` matches 128 billion queues.  The queue pattern
/// `*5*5*5*5` matches 40 trillion queues.  Twenty pieces is plenty for now.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct Queue(u64);

fn mask(idx: usize) -> u64 {
    (1 << idx) - 1
}

impl rdst::RadixKey for Queue {
    const LEVELS: usize = 8;

    fn get_level(&self, level: usize) -> u8 {
        (self.0 >> (8 * level)) as u8
    }
}

impl Queue {
    pub fn new() -> Queue {
        Queue(0)
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn len(self) -> usize {
        20 - self.trailing() / 3
    }

    fn trailing(self) -> usize {
        let bounded = self.0 | (1 << 60);
        bounded.trailing_zeros() as usize
    }

    pub fn push(&mut self, s: Shape) {
        assert!(self.len() < 20);
        let shift = self.trailing() / 3 * 3 - 3;
        self.0 |= (s as u64 + 1) << shift;
    }

    pub fn pop(&mut self) -> Option<Shape> {
        let shift = self.trailing() / 3 * 3;
        let shape = decode(self.0 >> shift);

        let mask = (1 << (shift + 3)) - 1;
        self.0 &= !mask;

        shape
    }

    pub fn remove(&mut self, idx: usize) -> Option<Shape> {
        assert!(idx < 20);

        let begin = self.0 & mask(60) & !mask(60 - 3 * idx);
        let shape = decode(self.0 >> (57 - 3 * idx));
        let end = self.0 & mask(57 - 3 * idx);

        self.0 = begin | (end << 3);
        shape
    }

    pub fn without(self, idx: usize) -> (Shape, Queue) {
        let mut q = self;
        let s = q.remove(idx).unwrap();
        (s, q)
    }

    pub fn take_each(self) -> impl Iterator<Item = (Shape, Queue)> {
        struct Each(Queue, usize);
        impl Iterator for Each {
            type Item = (Shape, Queue);
            fn next(&mut self) -> Option<Self::Item> {
                if self.1 < self.0.len() {
                    self.1 += 1;
                    Some(self.0.without(self.1 - 1))
                } else {
                    None
                }
            }
        }
        Each(self, 0)
    }

    #[must_use]
    pub fn concat(self, other: &Queue) -> Queue {
        self.concat2(*other)
    }

    #[must_use]
    pub fn concat2(self, other: Queue) -> Queue {
        assert!(self.len() + other.len() <= 20);
        let shift = self.len() * 3;
        Queue(self.0 | (other.0 >> shift))
    }
}

fn decode(s: u64) -> Option<Shape> {
    match s & 7 {
        0 => None,
        1 => Some(Shape::I),
        2 => Some(Shape::J),
        3 => Some(Shape::L),
        4 => Some(Shape::O),
        5 => Some(Shape::S),
        6 => Some(Shape::T),
        7 => Some(Shape::Z),
        _ => unreachable!(),
    }
}

impl Iterator for Queue {
    type Item = Shape;

    fn next(&mut self) -> Option<Shape> {
        self.remove(0)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl DoubleEndedIterator for Queue {
    fn next_back(&mut self) -> Option<Shape> {
        self.pop()
    }
}

// this defines Queue::len and Queue::is_empty and makes a mess
// impl ExactSizeIterator for Queue {}

#[pymethods]
impl QueueSet {
    #[pyo3(signature = (*patterns))]
    #[new]
    fn new(patterns: Vec<&str>) -> PyResult<QueueSet> {
        let mut set = QueueSet {
            patterns: BTreeSet::new(),
            queues: AHashSet::new(),
        };

        for pattern in &patterns {
            set.add(pattern)?;
        }

        Ok(set)
    }

    fn add(&mut self, pattern: &str) -> PyResult<()> {
        static WHOLE_RE: OnceLock<Regex> = OnceLock::new();
        static BAG_RE: OnceLock<Regex> = OnceLock::new();
        let whole_re = WHOLE_RE
            .get_or_init(|| Regex::new(r"^([IJLOSTZ]|(\[[IJLOSTZ]+\]|\*)(p?(\d+))?)*$").unwrap());
        let bag_re = BAG_RE.get_or_init(|| {
            Regex::new(r"([IJLOSTZ])()|(?:\[([IJLOSTZ]+)\]|(\*))(?:p?(\d*))").unwrap()
        });

        if !whole_re.is_match(pattern) {
            return Err(PyValueError::new_err("invalid pattern"));
        }

        let bag_specs: Vec<(Queue, usize)> = bag_re
            .captures_iter(pattern)
            .map(QueueSet::parse_spec)
            .collect::<PyResult<_>>()?;

        if bag_specs.iter().map(|(_, len)| *len).sum::<usize>() > 20 {
            return Err(PyValueError::new_err("queue too long"));
        }

        let bags: Vec<Vec<Queue>> = bag_specs.iter().map(QueueSet::resolve_spec).collect();

        if bags.iter().map(|bag| bag.len()).product::<usize>() > 100_000_000 {
            return Err(PyValueError::new_err("queue set too large"));
        }

        self.add_from_bags(&bags);

        self.patterns.insert(pattern.to_string());
        Ok(())
    }

    fn __len__(&self) -> usize {
        self.queues.len()
    }

    fn __repr__(&self) -> String {
        let mut s = "QueueSet(".to_string();

        for (i, pattern) in self.patterns.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("\"{}\"", pattern));
        }

        s.push(')');
        s
    }

    fn to_list(&self) -> Vec<Queue> {
        let mut queues: Vec<Queue> = self.queues.iter().copied().collect();
        queues.radix_sort_unstable();
        queues
    }

    fn __getitem__(&self, idx: usize) -> PyResult<Queue> {
        // Queues aren't stored in order, so we need to sort them first.
        if idx < self.queues.len() {
            let mut queues: Vec<Queue> = self.queues.iter().copied().collect();
            queues.radix_sort_unstable();
            Ok(queues[idx])
        } else {
            Err(PyIndexError::new_err("queue set index too large"))
        }
    }

    fn __iter__(&self) -> OrderedQueueIterator {
        let mut queues: Vec<Queue> = self.queues.iter().copied().collect();
        queues.radix_sort_unstable();
        OrderedQueueIterator(queues.into_iter())
    }
}

impl QueueSet {
    // Utilities for ingesting patterns.

    fn parse_spec(captures: Captures) -> PyResult<(Queue, usize)> {
        let (_, [contents, len]) = captures.extract();

        let contents = if contents == "*" { "IJLOSTZ" } else { contents };
        let bag: Queue = contents.try_into()?;
        let len = len.parse::<usize>().unwrap_or(1);

        if len > contents.len() {
            return Err(PyValueError::new_err("not enough pieces in bag"));
        } else {
            Ok((bag, len))
        }
    }

    fn resolve_spec((queue, len): &(Queue, usize)) -> Vec<Queue> {
        fn inner(building: Queue, from: Queue, len: usize, into: &mut Vec<Queue>) {
            if len == 0 {
                into.push(building);
                return;
            }
            for (shape, left) in from.take_each() {
                let mut building = building;
                building.push(shape);
                inner(building, left, len - 1, into);
            }
        }

        let mut queues = Vec::new();
        inner(Queue::new(), *queue, *len, &mut queues);
        queues.radix_sort_unstable();
        queues.dedup();
        queues
    }

    fn add_from_bags(&mut self, bags: &[Vec<Queue>]) {
        fn inner(building: Queue, into: &mut AHashSet<Queue>, rest: &[Vec<Queue>]) {
            match rest {
                [] => {
                    into.insert(building);
                }
                [choice, rest @ ..] => {
                    for next in choice {
                        inner(building.concat2(*next), into, rest);
                    }
                }
            }
        }

        inner(Queue::new(), &mut self.queues, bags)
    }
}

#[pyclass]
struct OrderedQueueIterator(std::vec::IntoIter<Queue>);

#[pymethods]
impl OrderedQueueIterator {
    fn __iter__(slf: PyRef<Self>) -> PyRef<Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Queue> {
        self.0.next()
    }
}

impl TryFrom<&str> for Queue {
    type Error = PyErr;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() > 20 {
            return Err(PyValueError::new_err("queue too long"));
        }

        let mut queue = Queue::new();
        for b in value.bytes() {
            queue.push(match b {
                b'I' => Shape::I,
                b'J' => Shape::J,
                b'L' => Shape::L,
                b'O' => Shape::O,
                b'S' => Shape::S,
                b'T' => Shape::T,
                b'Z' => Shape::Z,
                _ => return Err(PyValueError::new_err("invalid shape")),
            });
        }

        Ok(queue)
    }
}

impl IntoPy<Py<PyAny>> for Queue {
    fn into_py(self, py: Python<'_>) -> Py<PyAny> {
        PyString::new(py, &self.conv::<String>()).into_py(py)
    }
}

impl From<Queue> for String {
    fn from(value: Queue) -> Self {
        value.into_iter().map(char::from).collect()
    }
}
