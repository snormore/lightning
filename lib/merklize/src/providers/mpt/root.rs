use atomo::{SerdeBackend, StorageBackend, TableRef, TableSelector};
use tracing::trace;

use super::tree::ROOT_TABLE_NAME;
use crate::StateRootHash;

/// A wrapper around the root table to provide a more ergonomic API for reading and writing the
/// state root hash.
pub(crate) struct RootTable<'a, B: StorageBackend, S: SerdeBackend> {
    table: TableRef<'a, u8, StateRootHash, B, S>,
}

impl<'a, B: StorageBackend, S: SerdeBackend> RootTable<'a, B, S> {
    pub fn new(ctx: &'a TableSelector<B, S>) -> Self {
        let table = ctx.get_table(ROOT_TABLE_NAME);
        Self { table }
    }

    /// Read the state root hash from the root table.
    pub fn get(&self) -> Option<StateRootHash> {
        // We only store the latest root hash in the root table, and so we just use the key 0u8.
        let root = self.table.get(0);
        trace!(?root, "get");
        root
    }

    /// Write the given state root to the root table.
    pub fn set(&mut self, root: StateRootHash) {
        // We only store the latest root hash in the root table, and so we just use the key 0u8.
        trace!(?root, "set");
        self.table.insert(0, root);
    }
}
