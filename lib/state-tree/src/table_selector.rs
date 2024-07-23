use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::{RootHash, SimpleHasher};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::jmt::JmtTreeReader;
use crate::{SerializedNodeKey, SerializedNodeValue, StateTreeStrategy, StateTreeTableRef};

pub struct StateTreeTableSelector<
    'a,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    X: StateTreeStrategy<B, S, KH, VH>,
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
    X: StateTreeStrategy<B, S, KH, VH>,
> StateTreeTableSelector<'a, B, S, KH, VH, X>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
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

    /// Returns the state tree table reference.
    #[inline]
    pub fn state_tree_table(&self) -> &TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S> {
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
    ) -> StateTreeTableRef<K, V, B, S, KH, VH, X>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        StateTreeTableRef::new(
            self.inner.get_table(name.clone()),
            self.strategy,
            name.as_ref().to_string(),
        )
    }

    /// Return the state root hash of the state tree.
    // TODO(snormore): This is leaking `jmt::RootHash`.`
    pub fn get_state_root(&self) -> Result<RootHash> {
        let reader = JmtTreeReader::new(self.strategy.tree_table());
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        tree.get_root_hash(0)
    }
}
