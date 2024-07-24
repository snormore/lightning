use std::any::Any;
use std::hash::Hash;

use atomo::{AtomoBuilder, StorageBackendConstructor, UpdatePerm};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::{SerializedTreeNodeKey, SerializedTreeNodeValue};
use crate::{MerklizedAtomo, MerklizedLayout};

const DEFAULT_STATE_TREE_TABLE_NAME: &str = "%state_tree_nodes";

/// This is a builder of `[crate::MerklizedAtomoWriter]` and `[crate::MerklizedAtomoReader]`
/// instances, that is used to initialize which tables are used and how they are configured.
///
/// It wraps the `[atomo::AtomoBuilder]` for building a `[crate::MerklizedAtomoWriter]`(a wrapper
/// of `[atomo::Atomo<UpdatePerm>]`), and can be used to build a `[crate::MerklizedAtomoReader]` (a
/// wrapper of `[atomo::Atomo<QueryPerm>]`).
pub struct MerklizedAtomoBuilder<C: StorageBackendConstructor, L: MerklizedLayout> {
    inner: AtomoBuilder<C, L::SerdeBackend>,
    tree_table_name: String,
}

impl<C: StorageBackendConstructor, L: MerklizedLayout> MerklizedAtomoBuilder<C, L> {
    /// Create a new builder with the given storage backend constructor.
    pub fn new(constructor: C) -> Self {
        Self {
            inner: AtomoBuilder::new(constructor),
            tree_table_name: DEFAULT_STATE_TREE_TABLE_NAME.to_string(),
        }
    }

    /// Open a new table with the given name and key-value type.
    /// This is a pass-through to `[atomo::AtomoBuilder::with_table]`.
    pub fn with_table<K, V>(self, table: impl ToString) -> Self
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        Self {
            inner: self.inner.with_table::<K, V>(table),
            ..self
        }
    }

    /// Enable key iteration on the table with the given name.
    /// This is a pass-through to `[atomo::AtomoBuilder::enable_iter]`.
    pub fn enable_iter(self, table: impl ToString) -> Self {
        Self {
            inner: self.inner.enable_iter(&table.to_string()),
            ..self
        }
    }

    /// Set the name of the table that will store the state tree, and return a new, updated builder.
    /// This is a pass-through to `[crate::MerklizedAtomoWriter::with_tree_table_name]`.
    pub fn with_tree_table_name(self, name: impl ToString) -> Self {
        Self {
            tree_table_name: name.to_string(),
            ..self
        }
    }

    /// Build and return a writer for the state tree.
    pub fn build(self) -> Result<MerklizedAtomo<UpdatePerm, C::Storage, L>, C::Error> {
        // TODO(snormore): Figure out a better way to get the table id by name.
        let table_id_by_name = self.inner.table_name_to_id();
        let atomo = self
            .inner
            .with_table::<SerializedTreeNodeKey, SerializedTreeNodeValue>(&self.tree_table_name)
            // TODO(snormore): No need to enable_iter on this table by default, it's just used in a
            // test right now
            .enable_iter(&self.tree_table_name)
            .build()?;
        Ok(MerklizedAtomo::new(
            atomo,
            self.tree_table_name,
            table_id_by_name,
        ))
    }
}
