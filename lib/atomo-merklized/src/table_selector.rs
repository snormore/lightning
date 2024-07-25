use std::any::Any;
use std::hash::Hash;

use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{StorageBackend, TableRef};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    MerklizedLayout,
    MerklizedStrategy,
    MerklizedTableRef,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateRootHash,
    StateTable,
};

/// A selector for tables in a merklized atomo, that can be used to query and update the tables. It
/// wraps an atomo table selector and a reference to the state tree table.
pub struct MerklizedTableSelector<'a, B: StorageBackend, L: MerklizedLayout> {
    inner: &'a atomo::TableSelector<B, L::SerdeBackend>,
    tree_table:
        &'a atomo::TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, L::SerdeBackend>,
}

impl<'a, B: StorageBackend, L: MerklizedLayout> MerklizedTableSelector<'a, B, L> {
    /// Create a new table selector.
    pub fn new(
        inner: &'a atomo::TableSelector<B, L::SerdeBackend>,
        tree_table: &'a atomo::TableRef<
            'a,
            SerializedTreeNodeKey,
            SerializedTreeNodeValue,
            B,
            L::SerdeBackend,
        >,
    ) -> Self {
        Self { inner, tree_table }
    }

    /// Returns the inner atomo table selector.
    #[inline]
    pub fn inner(&self) -> &'a atomo::TableSelector<B, L::SerdeBackend> {
        self.inner
    }

    /// Returns the state tree table reference.
    #[inline]
    pub fn state_tree_table(
        &self,
    ) -> &TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, L::SerdeBackend> {
        self.tree_table
    }

    /// Returns the current changes in the batch.
    #[inline]
    pub fn batch(&self) -> VerticalBatch {
        self.inner.batch()
    }

    /// Return the table reference for the table with the provided name and K, V type.
    #[inline]
    pub fn get_table<K, V>(&self, table: impl AsRef<str>) -> MerklizedTableRef<K, V, B, L>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        MerklizedTableRef::new(
            self.inner.get_table(table.as_ref()),
            self.tree_table,
            StateTable::new(table),
        )
    }

    /// Return the state root hash of the state tree.
    #[inline]
    pub fn get_state_root(&self) -> Result<StateRootHash> {
        L::Strategy::get_root::<B, L::SerdeBackend>(self.tree_table)
    }
}
