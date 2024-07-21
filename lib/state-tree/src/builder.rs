use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor};
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::types::{SerializedNodeKey, SerializedNodeValue};
use crate::StateTreeAtomo;

const TREE_TABLE_NAME: &str = "%state_tree_nodes";

pub struct StateTreeBuilder<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
> {
    atomo: AtomoBuilder<C, S>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<C: StorageBackendConstructor, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeBuilder<C, S, KH, VH>
where
    C::Storage: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    pub fn new(constructor: C) -> Self {
        Self {
            atomo: AtomoBuilder::new(constructor),
            _phantom: PhantomData,
        }
    }

    #[inline(always)]
    pub fn with_table<K, V>(self, name: impl ToString) -> Self
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        Self {
            atomo: self.atomo.with_table::<K, V>(name),
            ..self
        }
    }

    pub fn enable_iter(self, name: &str) -> Self {
        Self {
            atomo: self.atomo.enable_iter(name),
            ..self
        }
    }

    pub fn build(self) -> Result<StateTreeAtomo<C::Storage, S, KH, VH>, C::Error> {
        // TODO(snormore): Figure out a better way to get the table id by name.
        let table_id_by_name = self.atomo.table_name_to_id();
        let atomo = self
            .atomo
            .with_table::<SerializedNodeKey, SerializedNodeValue>(TREE_TABLE_NAME)
            // TODO(snormore): No need to enable_iter on this table by default
            .enable_iter(TREE_TABLE_NAME)
            .build()?;
        Ok(StateTreeAtomo::new(
            atomo,
            TREE_TABLE_NAME.to_string(),
            table_id_by_name,
        ))
    }
}
