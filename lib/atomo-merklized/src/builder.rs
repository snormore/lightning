use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor};
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::{SerializedNodeKey, SerializedNodeValue};
use crate::MerklizedAtomoWriter;

const DEFAULT_STATE_TREE_TABLE_NAME: &str = "%state_tree_nodes";

pub struct MerklizedAtomoBuilder<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedAtomoStrategy<C::Storage, S, KH, VH>,
> {
    inner: AtomoBuilder<C, S>,
    tree_table_name: String,
    _phantom: PhantomData<(KH, VH)>,
}

impl<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedAtomoStrategy<C::Storage, S, KH, VH>,
> MerklizedAtomoBuilder<C, S, KH, VH>
where
    C::Storage: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    pub fn new(constructor: C) -> Self {
        Self {
            inner: AtomoBuilder::new(constructor),
            tree_table_name: DEFAULT_STATE_TREE_TABLE_NAME.to_string(),
            _phantom: PhantomData,
        }
    }

    pub fn with_table<K, V>(self, name: impl ToString) -> Self
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        Self {
            inner: self.inner.with_table::<K, V>(name),
            ..self
        }
    }

    pub fn enable_iter(self, name: impl ToString) -> Self {
        Self {
            inner: self.inner.enable_iter(name.to_string().as_str()),
            ..self
        }
    }

    /// Set the name of the table that will store the state tree, and return a new, updated builder.
    pub fn with_tree_table_name(self, name: impl ToString) -> Self {
        Self {
            tree_table_name: name.to_string(),
            ..self
        }
    }

    /// Build and return a writer for the state tree.
    pub fn build(self) -> Result<MerklizedAtomoWriter<C::Storage, S, KH, VH>, C::Error> {
        // TODO(snormore): Figure out a better way to get the table id by name.
        let table_id_by_name = self.inner.table_name_to_id();
        let atomo = self
            .inner
            .with_table::<SerializedNodeKey, SerializedNodeValue>(&self.tree_table_name)
            // TODO(snormore): No need to enable_iter on this table by default
            .enable_iter(&self.tree_table_name)
            .build()?;
        Ok(MerklizedAtomoWriter::<C::Storage, S, KH, VH>::new(
            atomo,
            self.tree_table_name,
            table_id_by_name,
        ))
    }
}
