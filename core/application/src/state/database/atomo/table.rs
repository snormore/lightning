use std::borrow::Borrow;

use atomo::{KeyIterator, SerdeBackend, StorageBackend, TableRef};
use serde::Serialize;

use crate::database::DatabaseTable;

pub struct AtomoDatabaseTable<'ctx, K: Serialize, V: Serialize, B: StorageBackend, S: SerdeBackend>
{
    inner: TableRef<'ctx, K, V, B, S>,
}

impl<'ctx, K: Serialize, V: Serialize, B: StorageBackend, S: SerdeBackend> DatabaseTable
    for AtomoDatabaseTable<'ctx, K, V, B, S>
{
    type Key = K;
    type Value = V;

    fn insert(&mut self, key: impl Borrow<K>, value: impl Borrow<V>) {
        self.inner.insert(key, value)
    }

    fn remove(&mut self, key: impl Borrow<K>) {
        self.inner.remove(key)
    }

    fn get(&self, key: impl Borrow<K>) -> Option<V> {
        self.inner.get(key)
    }

    fn contains_key(&self, key: impl Borrow<K>) -> bool {
        self.inner.contains_key(key)
    }

    fn keys(&self) -> KeyIterator<K> {
        self.inner.keys()
    }
}
