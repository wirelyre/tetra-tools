use crossbeam::utils::CachePadded;
use std::sync::atomic::{AtomicU64, Ordering};

/// [`rayon`]-aware counter.  Starts at zero, counts up.
///
/// This is **only** for user feedback in the middle of a long multi-core
/// computation.
///
/// This uses as many internal counters as the number of CPUs, as determined by
/// the [`num_cpus`] crate.  When accessed from [`rayon`] threads, ensure that
/// there are no more threads than CPUs.
///
/// Accesses are all done with [`Relaxed`](Ordering::Relaxed) ordering.  The
/// only guarantee you have from this structure is:
///
///   1. If increments do not continue forever, then
///   2. sometime [`Counter::get`] will return the final number of increments.
///
/// Note that this could happen before or after incrementation is done.
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
