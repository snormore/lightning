use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::{RootHash, SimpleHasher};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::jmt::JmtTreeReader;
use crate::{SerializedNodeKey, SerializedNodeValue, StateTreeTableRef};

pub struct StateTreeTableSelector<
    'a,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
> {
    inner: &'a atomo::TableSelector<B, S>,
    tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<'a, B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeTableSelector<'a, B, S, KH, VH>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    /// Create a new table selector.
    #[inline]
    pub fn new(
        inner: &'a atomo::TableSelector<B, S>,
        tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    ) -> Self {
        Self {
            inner,
            tree_table,
            _phantom: PhantomData,
        }
    }

    /// Returns the state tree table reference.
    #[inline]
    pub fn state_tree_table(&self) -> &TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S> {
        self.tree_table
    }

    /// Returns the current changes in the batch.
    #[inline]
    pub fn current_changes(&self) -> VerticalBatch {
        self.inner.current_changes()
    }

    /// Return the table reference for the table with the provided name and K, V type.
    pub fn get_table<K, V>(
        &self,
        name: impl AsRef<str> + Clone,
    ) -> StateTreeTableRef<K, V, B, S, KH, VH>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any + Debug,
    {
        StateTreeTableRef::new(
            self.inner.get_table(name.clone()),
            name.as_ref().to_string(),
            self.tree_table,
        )
    }

    /// Return the state root hash of the state tree.
    // TODO(snormore): This is leaking `jmt::RootHash`.`
    pub fn get_state_root(&self) -> Result<RootHash> {
        let reader = JmtTreeReader::new(self.tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        tree.get_root_hash(0)
    }
}
