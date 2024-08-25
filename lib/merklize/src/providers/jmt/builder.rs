use std::marker::PhantomData;

use atomo::AtomoBuilder;
use jmt::storage::{Node, NodeKey};
use jmt::KeyHash;

use super::tree::{KEYS_TABLE_NAME, NODES_TABLE_NAME};
use crate::{StateKey, StateTree, StateTreeBuilder};

#[derive(Debug, Clone)]
pub struct JmtStateTreeBuilder<T: StateTree> {
    _tree: PhantomData<T>,
}

impl<T: StateTree> JmtStateTreeBuilder<T> {
    pub fn new() -> Self {
        Self { _tree: PhantomData }
    }
}

impl<T: StateTree> Default for JmtStateTreeBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: StateTree> StateTreeBuilder<T> for JmtStateTreeBuilder<T> {
    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn register_tables(
        builder: AtomoBuilder<T::StorageBuilder, T::Serde>,
    ) -> AtomoBuilder<T::StorageBuilder, T::Serde> {
        builder
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
    }
}
