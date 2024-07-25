use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;

use atomo::{KeyIterator, SerdeBackend, StorageBackend};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    MerklizedLayout,
    MerklizedStrategy,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateTable,
};

pub struct MerklizedTableRef<
    'a,
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any,
    B: StorageBackend,
    L: MerklizedLayout,
> {
    inner: atomo::TableRef<'a, K, V, B, L::SerdeBackend>,
    tree_table:
        &'a atomo::TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, L::SerdeBackend>,
    table: StateTable,
}

impl<'a, K, V, B: StorageBackend, L: MerklizedLayout> MerklizedTableRef<'a, K, V, B, L>
where
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any,
{
    /// Create a new table reference.
    pub fn new(
        inner: atomo::TableRef<'a, K, V, B, L::SerdeBackend>,
        tree_table: &'a atomo::TableRef<
            SerializedTreeNodeKey,
            SerializedTreeNodeValue,
            B,
            L::SerdeBackend,
        >,
        table: StateTable,
    ) -> Self {
        Self {
            inner,
            tree_table,
            table,
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
    // TODO(snormore): Return a proof type instead of a `Vec<u8>`, or something standard like an
    // ics23 proof.
    pub fn get_with_proof(&self, key: impl Borrow<K>) -> (Option<V>, Vec<u8>) {
        let value = self
            .get(key.borrow())
            .map(|value| L::SerdeBackend::serialize(&value));
        let key = L::SerdeBackend::serialize(key.borrow());
        let (value, proof) = L::Strategy::get_proof::<B, L::SerdeBackend>(
            self.tree_table,
            self.table.clone(),
            key.into(),
            value.map(Into::into),
        )
        .unwrap();

        let value = value.map(|value| L::SerdeBackend::deserialize::<V>(value.as_bytes()));
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
