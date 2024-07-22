use std::any::Any;
use std::borrow::Borrow;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{KeyIterator, SerdeBackend, StorageBackend, TableRef};
use jmt::proof::SparseMerkleProof;
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::jmt::JmtTreeReader;
use crate::{SerializedNodeKey, SerializedNodeValue, TableKey};

pub struct StateTreeTableRef<
    'a,
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
> {
    inner: atomo::TableRef<'a, K, V, B, S>,
    table_name: String,
    tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<'a, K, V, B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeTableRef<'a, K, V, B, S, KH, VH>
where
    K: Hash + Eq + Serialize + DeserializeOwned + Any,
    V: Serialize + DeserializeOwned + Any + Debug,
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    /// Create a new table reference.
    pub fn new(
        inner: atomo::TableRef<'a, K, V, B, S>,
        table_name: String,
        tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    ) -> Self {
        Self {
            inner,
            table_name,
            tree_table,
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
    pub fn get_with_proof(&self, key: impl Borrow<K>) -> (Option<V>, SparseMerkleProof<VH>) {
        let value = self.get(key.borrow());

        let reader = JmtTreeReader::new(self.tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        // Cache the value in the reader so it can be used when `get_with_proof` is called next,
        // which calls `get_value_option`, which needs the value mapping.
        let key = TableKey {
            table: self.table_name.clone(),
            key: S::serialize(key.borrow()),
        };
        let key_hash = key.hash::<KH>();
        if let Some(value) = value {
            reader.cache(key_hash, S::serialize(&value));
        }

        // TODO(snormore): Fix this unwrap.
        let (value, proof) = tree.get_with_proof(key_hash, 0).unwrap();

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
