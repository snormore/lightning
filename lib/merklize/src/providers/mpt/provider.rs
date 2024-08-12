use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use atomo::batch::Operation;
use atomo::{
    Atomo,
    AtomoBuilder,
    InMemoryStorage,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableId,
    TableRef,
    TableSelector,
    UpdatePerm,
};
use fxhash::FxHashMap;
use tracing::{trace, trace_span};
use trie_db::proof::generate_proof;
use trie_db::{DBValue, TrieDBMutBuilder, TrieHash, TrieMut};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::layout::TrieLayoutWrapper;
use super::MptStateProof;
use crate::{MerklizeProvider, SimpleHasher, StateKey, StateRootHash};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const ROOT_TABLE_NAME: &str = "%state_tree_root";

pub(crate) type SharedNodesTableRef<'a, B, S, H> =
    Arc<Mutex<TableRef<'a, <SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue, B, S>>>;

type SharedRootTable<'a, B, S> = Arc<Mutex<RootTable<'a, B, S>>>;

#[derive(Debug, Clone)]
/// A merklize provider that uses a Merkle Patricia Trie (MPT) implementation ([`mpt`]) to manage
/// the database-backed state tree.
pub struct MptMerklizeProvider<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    _phantom: PhantomData<(B, S, H)>,
}

impl<B, S, H> MptMerklizeProvider<B, S, H>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }

    /// Get the state root hash of the state tree from the root table if it exists, or compute it
    /// from the state tree nodes table if it does not, and save it to the root table, before
    /// returning it.
    fn state_root(
        nodes_table: SharedNodesTableRef<B, S, H>,
        root_table: SharedRootTable<B, S>,
    ) -> Result<StateRootHash> {
        let root = { root_table.lock().unwrap().get() };
        if let Some(root) = root {
            Ok(root)
        } else {
            let mut root: <SimpleHasherWrapper<H> as hash_db::Hasher>::Out =
                StateRootHash::default().into();
            let mut adapter = Adapter::<B, S, H>::new(nodes_table.clone());
            let mut tree =
                TrieDBMutBuilder::<TrieLayoutWrapper<H>>::new(&mut adapter, &mut root).build();

            // Note that tree.root() calls tree.commit() before returning the root hash.
            let root = *tree.root();

            // Save the root hash to the root table.
            root_table.lock().unwrap().set(root.into());

            Ok(root.into())
        }
    }

    /// Update the state tree with the given batch of changes as an iterator.
    fn update_state_tree_from_iter<'a, I>(
        nodes_table: SharedNodesTableRef<'a, B, S, H>,
        root_table: SharedRootTable<'a, B, S>,
        batch: HashMap<String, I>,
    ) -> Result<()>
    where
        I: Iterator<Item = (Box<[u8]>, Operation)>,
    {
        // Get the current state root hash.
        let mut state_root: <SimpleHasherWrapper<H> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table.clone())?.into();

        // Initialize a `TrieDBMutBuilder` to update the state tree.
        let mut adapter = Adapter::<B, S, H>::new(nodes_table.clone());
        let mut tree =
            TrieDBMutBuilder::<TrieLayoutWrapper<H>>::from_existing(&mut adapter, &mut state_root)
                .build();

        // Apply the changes in the batch to the state tree.
        for (table, changes) in batch {
            if table == NODES_TABLE_NAME || table == ROOT_TABLE_NAME {
                continue;
            }

            for (key, operation) in changes {
                let state_key = StateKey::new(&table, key.to_vec());
                let key_hash = state_key.hash::<S, H>();

                match operation {
                    Operation::Remove => {
                        trace!(?table, ?key_hash, "operation/remove");
                        tree.remove(key_hash.as_ref())?;
                    },
                    Operation::Insert(value) => {
                        trace!(?table, ?key_hash, ?value, "operation/insert");
                        tree.insert(key_hash.as_ref(), &value)?;
                    },
                }
            }
        }

        // Commit the changes to the state tree.
        {
            let span = trace_span!("triedb.commit");
            let _enter = span.enter();

            // Note that tree.root() calls tree.commit() before returning the root hash, so we don't
            // need to explicitly `tree.commit()` here, but otherwise would.
            let root = *tree.root();

            // Save the root hash to the root table.
            root_table.lock().unwrap().set(root.into());
        }

        Ok(())
    }
}

impl<B, S, H> Default for MptMerklizeProvider<B, S, H>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S, H> MerklizeProvider for MptMerklizeProvider<B, S, H>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    type Storage = B;
    type Serde = S;
    type Hasher = H;
    type Proof = MptStateProof;

    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn with_tables<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> AtomoBuilder<C, S> {
        builder
            .with_table::<<SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue>(
                NODES_TABLE_NAME,
            )
            .with_table::<u8, StateRootHash>(ROOT_TABLE_NAME)
    }

    /// Get the state root hash of the state tree.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_root(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<StateRootHash> {
        let span = trace_span!("get_state_root");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        Self::state_root(nodes_table, root_table)
    }

    /// Get an existence proof for the given key hash, if it is present in the state tree, or
    /// non-existence proof if it is not present.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_proof(
        ctx: &TableSelector<Self::Storage, Self::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<MptStateProof> {
        let span = trace_span!("get_state_proof");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        let state_root: <SimpleHasherWrapper<H> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table)?.into();
        let adapter = Adapter::<B, S, H>::new(nodes_table.clone());

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<S, H>();
        trace!(?key_hash, ?state_key, "get_state_proof");
        let key_hash: TrieHash<TrieLayoutWrapper<H>> = key_hash.into();

        let proof =
            generate_proof::<_, TrieLayoutWrapper<H>, _, _>(&adapter, &state_root, &vec![key_hash])
                .unwrap();
        let proof = MptStateProof::new(proof);

        Ok(proof)
    }

    /// Apply the state tree changes based on the state changes in the atomo batch. This will update
    /// the state tree to reflect the changes in the atomo batch.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn update_state_tree(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<()> {
        let span = trace_span!("update_state_tree");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        // Build a map of table indexes to table names.
        let tables = ctx.tables();
        let mut table_name_by_id = FxHashMap::default();
        for (i, table) in tables.iter().enumerate() {
            let table_id: TableId = i.try_into().unwrap();
            table_name_by_id.insert(table_id, table.clone());
        }

        let mut batch = HashMap::new();
        for (table_id, changes) in ctx.batch().into_raw().into_iter().enumerate() {
            let table_name = table_name_by_id.get(&(table_id as u8)).unwrap();
            batch.insert(
                table_name.clone(),
                changes.into_iter().map(|(k, v)| (k.clone(), v.clone())),
            );
        }

        Self::update_state_tree_from_iter(nodes_table, root_table, batch)
    }

    /// Build a temporary state tree from the full state, and return the root hash.
    fn build_state_root(db: &mut Atomo<UpdatePerm, B, S>) -> Result<StateRootHash> {
        let span = trace_span!("build_state_root");
        let _enter = span.enter();

        let tables = db.tables();
        let storage = db.get_storage_backend_unsafe();

        let mut batch = HashMap::new();
        for (i, table) in tables.into_iter().enumerate() {
            let tid = i as u8;

            let span = trace_span!("table");
            let _enter = span.enter();
            trace!(table, "build_state_root");

            let mut changes = Vec::new();

            for (key, value) in storage.get_all(tid) {
                let state_key = StateKey::new(&table, key.to_vec());

                trace!(?state_key, ?value, "key/value");

                changes.push((key, Operation::Insert(value)));
            }

            batch.insert(table, changes.into_iter());
        }

        type T<S, H> = MptMerklizeProvider<InMemoryStorage, S, H>;
        let builder = AtomoBuilder::new(InMemoryStorage::default());
        let mut tmp = T::<S, H>::with_tables(builder).build().unwrap();
        let root = tmp.run(|ctx| {
            let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
            let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

            T::<S, H>::update_state_tree_from_iter(nodes_table, root_table, batch)?;
            T::<S, H>::get_state_root(ctx)
        })?;

        Ok(root)
    }
}

/// A wrapper around the root table to provide a more ergonomic API for reading and writing the
/// state root hash.
struct RootTable<'a, B: StorageBackend, S: SerdeBackend> {
    table: TableRef<'a, u8, StateRootHash, B, S>,
}

impl<'a, B: StorageBackend, S: SerdeBackend> RootTable<'a, B, S> {
    fn new(ctx: &'a TableSelector<B, S>) -> Self {
        let table = ctx.get_table(ROOT_TABLE_NAME);
        Self { table }
    }

    /// Read the state root hash from the root table.
    fn get(&self) -> Option<StateRootHash> {
        // We only store the latest root hash in the root table, and so we just use the key 0u8.
        let root = self.table.get(0);
        trace!(?root, "get");
        root
    }

    /// Write the given state root to the root table.
    fn set(&mut self, root: StateRootHash) {
        // We only store the latest root hash in the root table, and so we just use the key 0u8.
        trace!(?root, "set");
        self.table.insert(0, root);
    }
}
