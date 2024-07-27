use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::Result;
use atomo::{
    Atomo,
    QueryPerm,
    ResolvedTableReference,
    SerdeBackend,
    StorageBackend,
    TableSelector,
    UpdatePerm,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{MerklizedStrategy, StateRootHash};

/// A merklized atomo, that can be used to query and update tables. It wraps an atomo instance and
/// a reference to the state tree table. It is parameterized by the permission type, which can be
/// either `UpdatePerm` or `QueryPerm`.
pub struct MerklizedAtomo<P, B: StorageBackend, S: SerdeBackend, M: MerklizedStrategy> {
    inner: Atomo<P, B, S>,
    _phantom: PhantomData<M>,
}

impl<B: StorageBackend, S: SerdeBackend, M: MerklizedStrategy> Clone
    for MerklizedAtomo<QueryPerm, B, S, M>
{
    fn clone(&self) -> Self {
        Self::new(self.inner.clone())
    }
}

impl<P, B: StorageBackend, S: SerdeBackend, M: MerklizedStrategy> MerklizedAtomo<P, B, S, M> {
    /// Create a new merklized atomo.
    pub fn new(inner: Atomo<P, B, S>) -> Self {
        Self {
            inner,
            _phantom: PhantomData,
        }
    }

    /// Build and return a query reader for the data.
    #[inline]
    pub fn query(&self) -> MerklizedAtomo<QueryPerm, B, S, M> {
        MerklizedAtomo::new(self.inner.query())
    }

    /// Resolve a table with the given name and key-value types.
    #[inline]
    pub fn resolve<K, V>(&self, name: impl AsRef<str>) -> ResolvedTableReference<K, V>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        self.inner.resolve::<K, V>(name.as_ref())
    }
}

impl<B: StorageBackend, S: SerdeBackend, M: MerklizedStrategy<Storage = B, Serde = S>>
    MerklizedAtomo<UpdatePerm, B, S, M>
{
    /// Run an update on the data.
    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut TableSelector<B, S>) -> R,
    {
        self.inner.run(|ctx| {
            let res = mutation(ctx);

            M::context(ctx).apply_state_tree_changes().unwrap();

            res
        })
    }

    /// Return the internal storage backend.
    pub fn get_storage_backend_unsafe(&mut self) -> &B {
        self.inner.get_storage_backend_unsafe()
    }
}

impl<B: StorageBackend, S: SerdeBackend, M: MerklizedStrategy<Storage = B, Serde = S>>
    MerklizedAtomo<QueryPerm, B, S, M>
{
    /// Run a query on the database.
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut TableSelector<B, S>) -> R,
    {
        self.inner.run(|ctx| query(ctx))
    }

    /// Return the state root hash of the state tree.
    #[inline]
    pub fn get_state_root(&self) -> Result<StateRootHash> {
        self.run(|ctx| M::context(ctx).get_state_root())
    }
}
