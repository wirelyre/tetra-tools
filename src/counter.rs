use crossbeam::utils::CachePadded;
use std::sync::atomic::{AtomicU64, Ordering};

pub struct Counter(Vec<CachePadded<AtomicU64>>);

impl Counter {
    pub fn zero() -> Counter {
        let mut vec = Vec::new();

        vec.resize_with(num_cpus::get(), || CachePadded::new(AtomicU64::new(0)));

        Counter(vec)
    }

    pub fn get(&self) -> u64 {
        self.0
            .iter()
            .map(|atomic| atomic.load(Ordering::Relaxed))
            .sum()
    }

    pub fn increment(&self) {
        let idx = rayon::current_thread_index().unwrap_or(0);
        self.0[idx].fetch_add(1, Ordering::Relaxed);
    }
}
