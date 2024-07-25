use std::any::Any;
use std::hash::Hash;

use anyhow::Result;
use atomo::{Atomo, QueryPerm, StorageBackend, TableIndex, UpdatePerm};
use fxhash::FxHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::{SerializedTreeNodeKey, SerializedTreeNodeValue};
use crate::{
    MerklizedLayout,
    MerklizedResolvedTableReference,
    MerklizedStrategy,
    MerklizedTableSelector,
    StateRootHash,
    StateTable,
};

/// A merklized atomo, that can be used to query and update tables. It wraps an atomo instance and
/// a reference to the state tree table. It is parameterized by the permission type, which can be
/// either `UpdatePerm` or `QueryPerm`.
// TODO(snormore): This is leaking `jmt::SimpleHasher`.
pub struct MerklizedAtomo<P, B: StorageBackend, L: MerklizedLayout> {
    inner: Atomo<P, B, L::SerdeBackend>,
    tree_table_name: String,
    table_name_by_id: FxHashMap<TableIndex, String>,
    table_id_by_name: FxHashMap<String, TableIndex>,
}

impl<B: StorageBackend, L: MerklizedLayout> Clone for MerklizedAtomo<QueryPerm, B, L> {
    fn clone(&self) -> Self {
        Self::new(
            self.inner.clone(),
            self.tree_table_name.clone(),
            self.table_id_by_name.clone(),
        )
    }
}

impl<P, B: StorageBackend, L: MerklizedLayout> MerklizedAtomo<P, B, L> {
    /// Create a new merklized atomo.
    pub fn new(
        inner: Atomo<P, B, L::SerdeBackend>,
        tree_table_name: String,
        table_id_by_name: FxHashMap<String, TableIndex>,
    ) -> Self {
        let table_name_by_id = table_id_by_name
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<FxHashMap<TableIndex, String>>();
        Self {
            inner,
            tree_table_name,
            table_name_by_id,
            table_id_by_name,
        }
    }

    /// Build and return a query reader for the data.
    #[inline]
    pub fn query(&self) -> MerklizedAtomo<QueryPerm, B, L> {
        MerklizedAtomo::new(
            self.inner.query(),
            self.tree_table_name.clone(),
            self.table_id_by_name.clone(),
        )
    }

    /// Resolve a table with the given name and key-value types.
    #[inline]
    pub fn resolve<K, V>(&self, name: impl AsRef<str>) -> MerklizedResolvedTableReference<K, V>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        MerklizedResolvedTableReference::new(
            self.inner.resolve::<K, V>(name.as_ref()),
            StateTable::new(name),
        )
    }
}

impl<B: StorageBackend, L: MerklizedLayout> MerklizedAtomo<UpdatePerm, B, L> {
    /// Run an update on the data.
    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut MerklizedTableSelector<B, L>) -> R,
    {
        let tree_table_name = self.tree_table_name.clone();
        self.inner.run(|ctx| {
            let mut tree_table =
                ctx.get_table::<SerializedTreeNodeKey, SerializedTreeNodeValue>(tree_table_name);
            let mut ctx = MerklizedTableSelector::<'_, B, L>::new(ctx, &tree_table);
            let res = mutation(&mut ctx);

            let batch = ctx.batch();

            #[allow(clippy::drop_non_drop)]
            drop(ctx);

            L::Strategy::apply_changes::<B, L::SerdeBackend>(
                &mut tree_table,
                self.table_name_by_id.clone(),
                batch,
            )
            .unwrap();

            res
        })
    }

    /// Return the internal storage backend.
    pub fn get_storage_backend_unsafe(&mut self) -> &B {
        self.inner.get_storage_backend_unsafe()
    }
}

impl<B: StorageBackend, L: MerklizedLayout> MerklizedAtomo<QueryPerm, B, L> {
    /// Run a query on the database.
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut MerklizedTableSelector<B, L>) -> R,
    {
        self.inner.run(|ctx| {
            let tree_table = ctx.get_table::<SerializedTreeNodeKey, SerializedTreeNodeValue>(
                self.tree_table_name.clone(),
            );
            let mut ctx = MerklizedTableSelector::new(ctx, &tree_table);
            query(&mut ctx)
        })
    }

    /// Return the state root hash of the state tree.
    #[inline]
    pub fn get_state_root(&self) -> Result<StateRootHash> {
        self.run(|ctx| ctx.get_state_root())
    }
}
