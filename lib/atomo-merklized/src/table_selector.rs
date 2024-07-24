use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{
    MerklizedStrategy,
    MerklizedTableRef,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateRootHash,
};

pub struct MerklizedTableSelector<
    'a,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    X: MerklizedStrategy<B, S, KH, VH>,
> {
    inner: &'a atomo::TableSelector<B, S>,
    strategy: &'a X,
    _phantom: PhantomData<(KH, VH)>,
}

impl<
    'a,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    X: MerklizedStrategy<B, S, KH, VH>,
> MerklizedTableSelector<'a, B, S, KH, VH, X>
{
    /// Create a new table selector.
    #[inline]
    pub fn new(inner: &'a atomo::TableSelector<B, S>, strategy: &'a X) -> Self {
        Self {
            inner,
            strategy,
            _phantom: PhantomData,
        }
    }

    /// Returns the inner atomo table selector.
    #[inline]
    pub fn inner(&self) -> &'a atomo::TableSelector<B, S> {
        self.inner
    }

    /// Returns the state tree table reference.
    #[inline]
    pub fn state_tree_table(
        &self,
    ) -> &TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, S> {
        self.strategy.tree_table()
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
    ) -> MerklizedTableRef<K, V, B, S, KH, VH, X>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        MerklizedTableRef::new(
            self.inner.get_table(name.clone()),
            self.strategy,
            name.as_ref().to_string(),
        )
    }

    /// Return the state root hash of the state tree.
    pub fn get_state_root(&self) -> Result<StateRootHash> {
        self.strategy.get_root_hash()
    }
}
