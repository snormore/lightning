use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{ensure, Result};
use atomo::batch::Operation;
use atomo::{
    Atomo,
    AtomoBuilder,
    InMemoryStorage,
    QueryPerm,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableId,
    TableSelector,
};
use fxhash::FxHashMap;
use tracing::{trace, trace_span};
use trie_db::proof::generate_proof;
use trie_db::{TrieDBMutBuilder, TrieHash, TrieMut};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::layout::TrieLayoutWrapper;
use super::tree::{
    RootTable,
    SharedNodesTableRef,
    SharedRootTable,
    NODES_TABLE_NAME,
    ROOT_TABLE_NAME,
};
use super::{MptStateProof, MptStateTree};
use crate::writer::StateTreeWriter;
use crate::{
    SimpleHasher,
    StateKey,
    StateRootHash,
    StateTree,
    StateTreeBuilder,
    StateTreeConfig,
    StateTreeReader,
    VerifyStateTreeError,
};

#[derive(Clone)]
pub struct MptStateTreeReader<C: StateTreeConfig> {
    db: Atomo<QueryPerm, <C::StorageBuilder as StorageBackendConstructor>::Storage, C::Serde>,
}

impl<C: StateTreeConfig> MptStateTreeReader<C>
where
    // Send + Sync bounds required by triedb/hashdb.
    C::StorageBuilder: StorageBackendConstructor + Send + Sync,
    <C::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    C::Serde: SerdeBackend + Send + Sync,
    C::Hasher: SimpleHasher + Send + Sync,
{
    /// Get the state root hash of the state tree from the root table if it exists, or compute it
    /// from the state tree nodes table if it does not, and save it to the root table, before
    /// returning it.
    // TODO(snormore): Remove/consolidate this duplicate method from `MptStateTreeWriter`.
    fn state_root(
        nodes_table: SharedNodesTableRef<
            <C::StorageBuilder as StorageBackendConstructor>::Storage,
            C::Serde,
            C::Hasher,
        >,
        root_table: SharedRootTable<
            <C::StorageBuilder as StorageBackendConstructor>::Storage,
            C::Serde,
        >,
    ) -> Result<StateRootHash> {
        let root = { root_table.lock().unwrap().get() };
        if let Some(root) = root {
            Ok(root)
        } else {
            let mut root: <SimpleHasherWrapper<C::Hasher> as hash_db::Hasher>::Out =
                StateRootHash::default().into();
            let mut adapter = Adapter::<
                <C::StorageBuilder as StorageBackendConstructor>::Storage,
                C::Serde,
                C::Hasher,
            >::new(nodes_table.clone());
            let mut tree =
                TrieDBMutBuilder::<TrieLayoutWrapper<C::Hasher>>::new(&mut adapter, &mut root)
                    .build();

            // Note that tree.root() calls tree.commit() before returning the root hash.
            let root = *tree.root();

            // Save the root hash to the root table.
            root_table.lock().unwrap().set(root.into());

            Ok(root.into())
        }
    }
}

impl<C: StateTreeConfig> StateTreeReader<C> for MptStateTreeReader<C>
where
    // Send + Sync bounds required
    // by triedb.
    C::StorageBuilder: StorageBackendConstructor + Send + Sync + Clone,
    <C::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync + Clone,
    C::Serde: SerdeBackend + Send + Sync + Clone,
    C::Hasher: SimpleHasher + Send + Sync + Clone,
{
    type Proof = MptStateProof;

    fn new(
        db: Atomo<
            QueryPerm,
            <<C as StateTreeConfig>::StorageBuilder as StorageBackendConstructor>::Storage,
            <C as StateTreeConfig>::Serde,
        >,
    ) -> Self {
        Self { db }
    }

    /// Get the state root hash of the state tree.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_root(
        ctx: &TableSelector<<C::StorageBuilder as StorageBackendConstructor>::Storage, C::Serde>,
    ) -> Result<StateRootHash> {
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
        ctx: &TableSelector<<C::StorageBuilder as StorageBackendConstructor>::Storage, C::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof> {
        let span = trace_span!("get_state_proof");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        let state_root: <SimpleHasherWrapper<C::Hasher> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table)?.into();
        let adapter = Adapter::<
            <C::StorageBuilder as StorageBackendConstructor>::Storage,
            C::Serde,
            C::Hasher,
        >::new(nodes_table.clone());

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<C::Serde, C::Hasher>();
        trace!(?key_hash, ?state_key, "get_state_proof");
        let key_hash: TrieHash<TrieLayoutWrapper<C::Hasher>> = key_hash.into();

        let proof = generate_proof::<_, TrieLayoutWrapper<C::Hasher>, _, _>(
            &adapter,
            &state_root,
            &vec![key_hash],
        )
        .unwrap();
        let proof = MptStateProof::new(proof);

        Ok(proof)
    }

    /// Verify that the state in the given atomo database instance, when used to build a new,
    /// temporary state tree from scratch, matches the stored state tree root hash.
    fn verify_state_tree_unsafe(
        db: &mut Atomo<
            QueryPerm,
            <C::StorageBuilder as StorageBackendConstructor>::Storage,
            C::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("verify_state_tree");
        let _enter = span.enter();

        // Build batch of all state data.
        let tables = db.tables();
        let mut batch = HashMap::new();
        for (i, table) in tables.clone().into_iter().enumerate() {
            let tid = i as u8;

            let mut changes = Vec::new();
            for (key, value) in db.get_storage_backend_unsafe().get_all(tid) {
                changes.push((key, Operation::Insert(value)));
            }
            batch.insert(table, changes.into_iter());
        }

        // Build a new, temporary state tree.
        let tmp_tree = MptStateTree::<
            InMemoryStorage,
            <C as StateTreeConfig>::Serde,
            <C as StateTreeConfig>::Hasher,
        >::new();
        let tmp_db = tmp_tree
            .builder(AtomoBuilder::new(InMemoryStorage::default()))
            .build()?;
        let tmp_query = tmp_db.reader();

        // Apply the batch to the temporary state tree.
        // TODO(snormore): The inner thing should be in the given context from run, a new type
        // called StateTreeContext.
        tmp_db.run(|ctx| tmp_db.update_state_tree(ctx, batch))?;

        // Get and return the state root hash from the temporary state tree.
        let tmp_state_root =
            tmp_query.run(|ctx| <TempStateTree<T> as StateTree>::Reader::get_state_root(ctx))?;

        // Check that the state root hash matches the stored state root hash.
        let stored_state_root = db.query().run(|ctx| Self::get_state_root(ctx))?;
        ensure!(
            tmp_state_root == stored_state_root,
            VerifyStateTreeError::StateRootMismatch(stored_state_root, tmp_state_root)
        );

        Ok(())
    }

    fn is_empty_state_tree_unsafe(
        db: &mut Atomo<
            QueryPerm,
            <C::StorageBuilder as StorageBackendConstructor>::Storage,
            C::Serde,
        >,
    ) -> Result<bool> {
        let span = trace_span!("is_empty_state_tree");
        let _enter = span.enter();

        let tables = db.tables();
        let table_id_by_name = tables
            .iter()
            .enumerate()
            .map(|(tid, table)| (table.clone(), tid as TableId))
            .collecC::<FxHashMap<_, _>>();

        let nodes_table_id = *table_id_by_name.get(NODES_TABLE_NAME).unwrap();
        let root_table_id = *table_id_by_name.get(ROOT_TABLE_NAME).unwrap();

        let storage = db.get_storage_backend_unsafe();

        // TODO(snormore): This should use iterators to avoid loading all keys into memory. We only
        // need to see if there is at least one key in each table, so `.next()` on an iterator
        // should be sufficient.
        Ok(storage.keys(nodes_table_id).len() == 0 && storage.keys(root_table_id).len() == 0)
    }
}
