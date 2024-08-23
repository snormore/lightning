use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use atomo::{
    AtomoBuilder,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableRef,
    TableSelector,
};
use tracing::trace;
use trie_db::DBValue;

use super::hasher::SimpleHasherWrapper;
use super::{MptStateProof, MptStateTreeBuilder, MptStateTreeReader, MptStateTreeWriter};
use crate::{SimpleHasher, StateRootHash, StateTree};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const ROOT_TABLE_NAME: &str = "%state_tree_root";

pub(crate) type SharedNodesTableRef<'a, B, S, H> =
    Arc<Mutex<TableRef<'a, <SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue, B, S>>>;

pub(crate) type SharedRootTable<'a, B, S> = Arc<Mutex<RootTable<'a, B, S>>>;

#[derive(Debug, Clone)]
pub struct MptStateTree<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> {
    _storage: PhantomData<B>,
    _serde: PhantomData<S>,
    _hasher: PhantomData<H>,
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> MptStateTree<B, S, H> {
    pub fn new() -> Self {
        Self {
            _storage: PhantomData,
            _serde: PhantomData,
            _hasher: PhantomData,
        }
    }
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> Default
    for MptStateTree<B, S, H>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S, H> StateTree for MptStateTree<B, S, H>
where
    // Send + Sync bounds required by triedb/hashdb.
    B: StorageBackendConstructor + Send + Sync,
    <B as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    type StorageBuilder = B;
    type Serde = S;
    type Hasher = H;
    type Proof = MptStateProof;

    type Builder = MptStateTreeBuilder<Self>;
    type Reader = MptStateTreeReader<Self>;
    type Writer = MptStateTreeWriter<Self>;

    fn new() -> Self {
        Self::new()
    }

    fn builder(self, builder: AtomoBuilder<Self::StorageBuilder, Self::Serde>) -> Self::Builder {
        MptStateTreeBuilder::new(builder)
    }
}

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
