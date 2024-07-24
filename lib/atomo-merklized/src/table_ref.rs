use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{KeyIterator, SerdeBackend, StorageBackend};
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::StateProof;
use crate::MerklizedStrategy;

pub struct MerklizedTableRef<
    'a,
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    X: MerklizedStrategy<B, S, KH, VH>,
> {
    inner: atomo::TableRef<'a, K, V, B, S>,
    strategy: &'a X,
    table_name: String,
    _phantom: PhantomData<(KH, VH)>,
}

impl<
    'a,
    K,
    V,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    X: MerklizedStrategy<B, S, KH, VH>,
> MerklizedTableRef<'a, K, V, B, S, KH, VH, X>
where
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any,
{
    /// Create a new table reference.
    pub fn new(
        inner: atomo::TableRef<'a, K, V, B, S>,
        strategy: &'a X,
        table_name: String,
    ) -> Self {
        Self {
            inner,
            strategy,
            table_name,
            _phantom: PhantomData,
        }
    }

    /// Insert a new `key` and `value` pair into the table.
    pub fn insert(&mut self, key: impl Borrow<K>, value: impl Borrow<V>) {
        self.inner.insert(key, value)
    }

    /// Remove the given key from the table.
    pub fn remove(&mut self, key: impl Borrow<K>) {
        self.inner.remove(key)
    }

    /// Returns the value associated with the provided key. If the key doesn't exits in the table
    /// [`None`] is returned.
    pub fn get(&self, key: impl Borrow<K>) -> Option<V> {
        self.inner.get(key)
    }

    /// Return the value associated with the provided key, along with a merkle proof of existence in
    /// the state tree. If the key doesn't exist in the table, [`None`] is returned.
    pub fn get_with_proof(&self, key: impl Borrow<K>) -> (Option<V>, StateProof<VH>) {
        let value = self.get(key.borrow()).map(|value| S::serialize(&value));
        let key = S::serialize(key.borrow());
        let (value, proof) = self
            .strategy
            .get_with_proof(self.table_name.clone(), key, value)
            .unwrap();

        let value = value.map(|value| S::deserialize::<V>(&value));
        (value, proof)
    }

    /// Returns `true` if the key exists in the table.
    pub fn contains_key(&self, key: impl Borrow<K>) -> bool {
        self.inner.contains_key(key)
    }

    /// Returns an iterator of the keys in this table.
    pub fn keys(&self) -> KeyIterator<K> {
        self.inner.keys()
    }
}
