use std::marker::PhantomData;

use atomo::AtomoBuilder;
use trie_db::DBValue;

use super::hasher::SimpleHasherWrapper;
use super::tree::{NODES_TABLE_NAME, ROOT_TABLE_NAME};
use crate::{StateRootHash, StateTree, StateTreeBuilder};

#[derive(Debug, Clone)]
pub struct MptStateTreeBuilder<T: StateTree> {
    _tree: PhantomData<T>,
}

impl<T: StateTree> MptStateTreeBuilder<T> {
    pub fn new() -> Self {
        Self { _tree: PhantomData }
    }
}

impl<T: StateTree> StateTreeBuilder<T> for MptStateTreeBuilder<T> {
    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn register_tables(
        builder: AtomoBuilder<T::StorageBuilder, T::Serde>,
    ) -> AtomoBuilder<T::StorageBuilder, T::Serde> {
        builder
            .with_table::<<SimpleHasherWrapper<T::Hasher> as hash_db::Hasher>::Out, DBValue>(
                NODES_TABLE_NAME,
            )
            .with_table::<u8, StateRootHash>(ROOT_TABLE_NAME)
    }
}
