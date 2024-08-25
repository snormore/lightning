use std::collections::HashMap;
use std::marker::PhantomData;
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
use crate::{
    SimpleHasher,
    StateKey,
    StateRootHash,
    StateTree,
    StateTreeBuilder,
    StateTreeReader,
    StateTreeWriter,
    VerifyStateTreeError,
};

#[derive(Clone)]
pub struct MptStateTreeReader<T: StateTree> {
    _tree: PhantomData<T>,
}

impl<T: StateTree> MptStateTreeReader<T>
where
    // Send + Sync bounds required by triedb/hashdb.
    T::StorageBuilder: StorageBackendConstructor + Send + Sync,
    <T::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    T::Serde: SerdeBackend + Send + Sync,
    T::Hasher: SimpleHasher + Send + Sync,
{
    pub fn new() -> Self {
        Self { _tree: PhantomData }
    }

    /// Get the state root hash of the state tree from the root table if it exists, or compute it
    /// from the state tree nodes table if it does not, and save it to the root table, before
    /// returning it.
    // TODO(snormore): Remove/consolidate this duplicate method from `MptStateTreeWriter`.
    fn state_root(
        nodes_table: SharedNodesTableRef<
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
            T::Hasher,
        >,
        root_table: SharedRootTable<
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
        >,
    ) -> Result<StateRootHash> {
        let root = { root_table.lock().unwrap().get() };
        if let Some(root) = root {
            Ok(root)
        } else {
            let mut root: <SimpleHasherWrapper<T::Hasher> as hash_db::Hasher>::Out =
                StateRootHash::default().into();
            let mut adapter = Adapter::<
                <T::StorageBuilder as StorageBackendConstructor>::Storage,
                T::Serde,
                T::Hasher,
            >::new(nodes_table.clone());
            let mut tree =
                TrieDBMutBuilder::<TrieLayoutWrapper<T::Hasher>>::new(&mut adapter, &mut root)
                    .build();

            // Note that tree.root() calls tree.commit() before returning the root hash.
            let root = *tree.root();

            // Save the root hash to the root table.
            root_table.lock().unwrap().set(root.into());

            Ok(root.into())
        }
    }
}

impl<T: StateTree> StateTreeReader<T> for MptStateTreeReader<T>
where
    T: StateTree<Proof = MptStateProof>,
    // Send + Sync bounds required by triedb/hashdb.
    T::StorageBuilder: StorageBackendConstructor + Send + Sync,
    <T::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    T::Serde: SerdeBackend + Send + Sync,
    T::Hasher: SimpleHasher + Send + Sync,
{
    /// Get the state root hash of the state tree.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_root(
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
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
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<T::Proof> {
        let span = trace_span!("get_state_proof");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        let state_root: <SimpleHasherWrapper<T::Hasher> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table)?.into();
        let adapter = Adapter::<
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
            T::Hasher,
        >::new(nodes_table.clone());

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<T::Serde, T::Hasher>();
        trace!(?key_hash, ?state_key, "get_state_proof");
        let key_hash: TrieHash<TrieLayoutWrapper<T::Hasher>> = key_hash.into();

        let proof = generate_proof::<_, TrieLayoutWrapper<T::Hasher>, _, _>(
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
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
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

        // Build a new, temporary state tree from the batch.
        type TempStateTree<T> =
            MptStateTree<InMemoryStorage, <T as StateTree>::Serde, <T as StateTree>::Hasher>;
        let builder = AtomoBuilder::<_, T::Serde>::new(InMemoryStorage::default());
        let mut tmp_db = <<TempStateTree<T> as StateTree>::Builder as StateTreeBuilder<
            TempStateTree<T>,
        >>::register_tables(builder)
        .build()?;
        tmp_db.run(|ctx| <TempStateTree<T> as StateTree>::Writer::update_state_tree(ctx, batch))?;

        // Get and return the state root hash from the temporary state tree.
        let tmp_state_root = tmp_db
            .query()
            .run(|ctx| <TempStateTree<T> as StateTree>::Reader::get_state_root(ctx))?;

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
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
        >,
    ) -> Result<bool> {
        let span = trace_span!("is_empty_state_tree");
        let _enter = span.enter();

        let tables = db.tables();
        let table_id_by_name = tables
            .iter()
            .enumerate()
            .map(|(tid, table)| (table.clone(), tid as TableId))
            .collect::<FxHashMap<_, _>>();

        let nodes_table_id = *table_id_by_name.get(NODES_TABLE_NAME).unwrap();
        let root_table_id = *table_id_by_name.get(ROOT_TABLE_NAME).unwrap();

        let storage = db.get_storage_backend_unsafe();

        // TODO(snormore): This should use iterators to avoid loading all keys into memory. We only
        // need to see if there is at least one key in each table, so `.next()` on an iterator
        // should be sufficient.
        Ok(storage.keys(nodes_table_id).len() == 0 && storage.keys(root_table_id).len() == 0)
    }
}
