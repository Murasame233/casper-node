//! A reference pool for items/objects.
//!
//! Its core responsibility is to deduplicate potentially expensive loads by keeping a weak
//! reference to any loaded object around, so that any load request for an object that is currently
//! in active use can be satisfied using the already existing copy.
//!
//! It differs from a cache in that it does not hold strong references to an item itself -- once an
//! item is no longer used, it will not be kept in the pool for a later request. As a consequence
//! the memory pool will never consume significantly more memory than what would otherwise be
//! required by the loaded objects that are in active use anyway and thus has an "infinite"
//! capacity.
use std::{
    borrow::Borrow,
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Weak},
};

use datasize::DataSize;

/// A pool of items/objects.
///
/// Maintains a pool of weak references and automatically purges them in configurable intervals.
///
/// # DataSize
///
/// Typically shared references like `Arc`s are not counted when using `DataSize`, however
/// `ObjectPool` counts its items in "regular" manner, as it is assumed to be the virtual owner.

#[derive(Debug)]
pub(super) struct ObjectPool<I> {
    /// The actual object pool.
    items: HashMap<I, Weak<[u8]>>,
    /// Interval for garbage collection, will remove dead references on every n-th `put()`.
    garbage_collect_interval: u16,
    /// Counts how many objects have been added since the last garbage collect interval.
    put_count: u16,
}

impl<I> ObjectPool<I> {
    /// Creates a new object pool.
    pub(super) fn new(garbage_collect_interval: u16) -> Self {
        Self {
            items: HashMap::new(),
            garbage_collect_interval,
            put_count: 0,
        }
    }
}

// Note: There is currently a design issue in the `datasize` crate where it does not gracefully
//       handle unsized types like slices, thus the derivation for any implementation of `DataSize
//       for Box<[T]>` based on `DataSize for Box<T>` and `DataSize for [T]` is bound to be
//       incorrect.
//
//       Since we currently only use very few different `T`s for `ObjectPool<T>`, we opt to
//       implement it manually here and gain a chance to optimize as well.
impl DataSize for ObjectPool<Box<[u8]>> {
    const IS_DYNAMIC: bool = true;

    const STATIC_HEAP_SIZE: usize = 0;

    fn estimate_heap_size(&self) -> usize {
        // See https://docs.rs/datasize/0.2.9/src/datasize/std.rs.html#213-224 for details.
        let base = self.items.capacity()
            * (size_of::<Box<[u8]>>() + size_of::<Weak<[u8]>>() + size_of::<usize>());

        base + self
            .items
            .iter()
            .map(|(key, value)| {
                // Unfortunately we have to check every instance by upgrading.
                let value_size = value.upgrade().map(|v| v.len()).unwrap_or_default();
                key.len() + value_size
            })
            .sum::<usize>()
    }
}

impl<I> ObjectPool<I>
where
    I: Hash + Eq,
{
    /// Stores a serialized object in the pool.
    ///
    /// At configurable intervals (see `garbage_collect_interval`), the entire pool will be checked
    /// and dead references pruned.
    pub(super) fn put(&mut self, id: I, item: Weak<[u8]>) {
        self.items.insert(id, item);

        if self.put_count >= self.garbage_collect_interval {
            self.items.retain(|_, item| item.strong_count() > 0);

            self.put_count = 0;
        }

        self.put_count += 1;
    }

    /// Retrieves an object from the pool, if present.
    pub(super) fn get<Q>(&self, id: &Q) -> Option<Arc<[u8]>>
    where
        I: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.items.get(id).and_then(Weak::upgrade)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use datasize::DataSize;

    use casper_types::Transaction;

    use super::ObjectPool;
    use crate::components::fetcher::FetchItem;

    impl<I> ObjectPool<I>
    where
        I: DataSize,
    {
        fn num_entries(&self) -> usize {
            self.items.len()
        }
    }

    #[test]
    fn can_load_and_store_items() {
        let mut pool: ObjectPool<<Transaction as FetchItem>::Id> = ObjectPool::new(5);
        let mut rng = crate::new_rng();

        let txn1 = Transaction::random(&mut rng);
        let txn2 = Transaction::random(&mut rng);
        let txn1_id = txn1.fetch_id();
        let txn2_id = txn2.fetch_id();
        let txn1_serialized = bincode::serialize(&txn1).expect("could not serialize first deploy");
        let txn2_serialized = bincode::serialize(&txn2).expect("could not serialize second deploy");

        let txn1_shared = txn1_serialized.into();
        let txn2_shared = txn2_serialized.into();

        assert!(pool.get(&txn1_id).is_none());
        assert!(pool.get(&txn2_id).is_none());

        pool.put(txn1_id, Arc::downgrade(&txn1_shared));
        assert!(Arc::ptr_eq(
            &pool.get(&txn1_id).expect("did not find d1"),
            &txn1_shared
        ));
        assert!(pool.get(&txn2_id).is_none());

        pool.put(txn2_id, Arc::downgrade(&txn2_shared));
        assert!(Arc::ptr_eq(
            &pool.get(&txn1_id).expect("did not find d1"),
            &txn1_shared
        ));
        assert!(Arc::ptr_eq(
            &pool.get(&txn2_id).expect("did not find d1"),
            &txn2_shared
        ));
    }

    #[test]
    fn frees_memory_after_reference_loss() {
        let mut pool: ObjectPool<<Transaction as FetchItem>::Id> = ObjectPool::new(5);
        let mut rng = crate::new_rng();

        let txn1 = Transaction::random(&mut rng);
        let txn1_id = txn1.fetch_id();
        let txn1_serialized = bincode::serialize(&txn1).expect("could not serialize first deploy");

        let txn1_shared = txn1_serialized.into();

        assert!(pool.get(&txn1_id).is_none());

        pool.put(txn1_id, Arc::downgrade(&txn1_shared));
        assert!(Arc::ptr_eq(
            &pool.get(&txn1_id).expect("did not find d1"),
            &txn1_shared
        ));

        drop(txn1_shared);
        assert!(pool.get(&txn1_id).is_none());
    }

    #[test]
    fn garbage_is_collected() {
        let mut pool: ObjectPool<<Transaction as FetchItem>::Id> = ObjectPool::new(5);
        let mut rng = crate::new_rng();

        assert_eq!(pool.num_entries(), 0);

        for i in 0..5 {
            let txn = Transaction::random(&mut rng);
            let id = txn.fetch_id();
            let serialized = bincode::serialize(&txn).expect("could not serialize first deploy");
            let shared = serialized.into();
            pool.put(id, Arc::downgrade(&shared));
            assert_eq!(pool.num_entries(), i + 1);
            drop(shared);
            assert_eq!(pool.num_entries(), i + 1);
        }

        let txn = Transaction::random(&mut rng);
        let id = txn.fetch_id();
        let serialized = bincode::serialize(&txn).expect("could not serialize first deploy");
        let shared = serialized.into();
        pool.put(id, Arc::downgrade(&shared));
        assert_eq!(pool.num_entries(), 1);
        drop(shared);
        assert_eq!(pool.num_entries(), 1);
    }
}
