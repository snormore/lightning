use std::any::Any;
use std::hash::Hash;

use atomo::{ResolvedTableReference, StorageBackend};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{MerklizedLayout, MerklizedTableRef, MerklizedTableSelector, StateTable};

#[derive(Clone)]
pub struct MerklizedResolvedTableReference<K, V> {
    inner: ResolvedTableReference<K, V>,
    table: StateTable,
}

impl<K, V> MerklizedResolvedTableReference<K, V> {
    pub fn new(inner: ResolvedTableReference<K, V>, table: StateTable) -> Self {
        Self { inner, table }
    }

    /// Returns the table reference for this table.
    ///
    /// # Panics
    ///
    /// If the table is already claimed.
    pub fn get<'selector, B: StorageBackend, L: MerklizedLayout>(
        &self,
        selector: &'selector MerklizedTableSelector<B, L>,
    ) -> MerklizedTableRef<'selector, K, V, B, L>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        let table_ref = MerklizedTableRef::new(
            self.inner.get(selector.inner()),
            selector.state_tree_table(),
            self.table.clone(),
        );
        table_ref
    }
}

impl<K, V> From<MerklizedResolvedTableReference<K, V>> for ResolvedTableReference<K, V> {
    fn from(resolved_table_ref: MerklizedResolvedTableReference<K, V>) -> Self {
        resolved_table_ref.inner
    }
}
