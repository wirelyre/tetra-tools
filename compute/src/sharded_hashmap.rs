use core::hash::{BuildHasher, Hash, Hasher};

use ahash::{AHashMap, RandomState};
use parking_lot::{Mutex, MutexGuard};
use rayon::prelude::*;

/// A concurrent hash map broken over many shards to allow fast access from
/// multiple cores.
///
/// The number of shards is `1 << SHARD_SIZE`.
///
/// Rust's ownership system makes working with this kind of data structure
/// somewhat awkward.  Mutable access to entries is possible by [holding a mutex
/// guard] for the containing shard, then accessing the inner hash map as
/// normal.
///
/// Common operations are performed on the sharded map directly.
///
/// Some operations take unique references.  This guarantees that the map is not
/// changing during the operation, and means that no mutexes are used.
///
/// The sister structure [`FrozenMap`] is for maps which are never intended to
/// change.  If using a map immutably over several threads, prefer `FrozenMap`
/// over collecting the contents of a `ShardedHashMap` into a different data
/// structure.
///
/// The hashing type parameter `H` only affects how shards are chosen.  The hash
/// map in each shard always uses [`ahash`].
///
/// [holding a mutex guard]: ShardedHashMap::get_shard_guard
pub struct ShardedHashMap<K, V, const SHARD_SIZE: usize, H = RandomState>(
    Vec<Mutex<AHashMap<K, V>>>,
    H,
)
where
    K: Hash + Eq + Send,
    V: Send,
    H: BuildHasher;

/// Immutable version of [`ShardedHashMap`].
///
/// This map can be constructed by [`ShardedHashMap::freeze`], or by collecting
/// from a parallel iterator directly (which does the same thing).
pub struct FrozenMap<K, V, const SHARD_SIZE: usize, H = RandomState>(Vec<AHashMap<K, V>>, H)
where
    K: Hash + Eq + Send,
    V: Send,
    H: BuildHasher;

fn hash<T: Hash, H: BuildHasher>(key: T, h: &H) -> u64 {
    let mut state = h.build_hasher();
    key.hash(&mut state);
    state.finish()
}

impl<K: Hash + Eq + Send, V: Send, const SHARD_SIZE: usize, H: BuildHasher>
    ShardedHashMap<K, V, SHARD_SIZE, H>
{
    pub fn new() -> Self
    where
        H: Default,
    {
        Self::new_with_hasher(H::default())
    }

    pub fn new_with_hasher(h: H) -> Self {
        let mut shards = Vec::new();
        for _ in 0..(1 << SHARD_SIZE) {
            shards.push(Mutex::new(AHashMap::new()));
        }
        ShardedHashMap(shards, h)
    }

    fn shard_idx(&self, key: &K) -> usize {
        let mask = (1 << SHARD_SIZE) - 1;
        (hash(key, &self.1) & mask) as usize
    }

    pub fn get_shard_guard(&self, key: &K) -> MutexGuard<'_, AHashMap<K, V>> {
        self.0[self.shard_idx(key)].lock()
    }

    /// Insert a `(key, value)` pair into the map.  Returns `None` if the key
    /// was not already present, or `Some(old)` if replacing `(key, old)`.
    pub fn insert(&self, key: K, value: V) -> Option<V> {
        self.get_shard_guard(&key).insert(key, value)
    }

    pub fn len(&mut self) -> usize {
        self.0
            .iter_mut()
            .map(|mutex| mutex.get_mut())
            .map(|shard| shard.len())
            .sum()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&K, &mut V)> {
        self.0.iter_mut().map(|mutex| mutex.get_mut()).flatten()
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let idx = self.shard_idx(key);
        self.0[idx].get_mut().get_mut(key)
    }

    /// Convert this map into an immutable map.  No locks will be necessary to
    /// access the immutable map.  This is much faster than collecting all the
    /// values into a new data structure.
    pub fn freeze(mut self) -> FrozenMap<K, V, SHARD_SIZE, H> {
        let shards = self.0.drain(..).map(|mutex| mutex.into_inner()).collect();
        FrozenMap(shards, self.1)
    }
}

impl<K: Hash + Eq + Send, V: Send, const SHARD_SIZE: usize, H: BuildHasher>
    FrozenMap<K, V, SHARD_SIZE, H>
{
    pub fn get(&self, key: &K) -> Option<&V> {
        let mask = (1 << SHARD_SIZE) - 1;
        let shard_idx = (hash(key, &self.1) & mask) as usize;
        self.0[shard_idx].get(key)
    }

    pub fn len(&self) -> usize {
        self.0.iter().map(|shard| shard.len()).sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.0.iter().flatten()
    }

    /// Make this map mutable.  Creates a mutex for each shard.
    pub fn thaw(mut self) -> ShardedHashMap<K, V, SHARD_SIZE, H> {
        let shards = self.0.drain(..).map(|shard| Mutex::new(shard)).collect();
        ShardedHashMap(shards, self.1)
    }
}

impl<'a, K, V, const SHARD_SIZE: usize, H> ParallelIterator
    for &'a mut ShardedHashMap<K, V, SHARD_SIZE, H>
where
    K: Hash + Eq + Send + Sync,
    V: Send,
    H: BuildHasher + Send + Sync,
{
    type Item = (&'a K, &'a mut V);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        self.0
            .par_iter_mut()
            .map(|mutex| mutex.get_mut())
            .flat_map(|shard| shard.par_iter_mut())
            .drive_unindexed(consumer)
    }
}

impl<K, V, const SHARD_SIZE: usize, H> FromParallelIterator<(K, V)>
    for ShardedHashMap<K, V, SHARD_SIZE, H>
where
    K: Hash + Eq + Send,
    V: Send,
    H: BuildHasher + Default + Sync,
{
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = (K, V)>,
    {
        let map = Self::new();

        par_iter.into_par_iter().for_each(|(k, v)| {
            map.insert(k, v);
        });

        map
    }
}

impl<'a, K, V, const SHARD_SIZE: usize, H> ParallelIterator for &'a FrozenMap<K, V, SHARD_SIZE, H>
where
    K: Hash + Eq + Send + Sync,
    V: Send + Sync,
    H: BuildHasher + Sync,
{
    type Item = (&'a K, &'a V);

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: rayon::iter::plumbing::UnindexedConsumer<Self::Item>,
    {
        self.0
            .par_iter()
            .flat_map(|shard| shard.par_iter())
            .drive_unindexed(consumer)
    }
}

impl<K, V, const SHARD_SIZE: usize, H> FromParallelIterator<(K, V)>
    for FrozenMap<K, V, SHARD_SIZE, H>
where
    K: Hash + Eq + Send,
    V: Send,
    H: BuildHasher + Default + Sync,
{
    fn from_par_iter<I>(par_iter: I) -> Self
    where
        I: IntoParallelIterator<Item = (K, V)>,
    {
        let map = ShardedHashMap::new();

        par_iter.into_par_iter().for_each(|(k, v)| {
            map.insert(k, v);
        });

        map.freeze()
    }
}
