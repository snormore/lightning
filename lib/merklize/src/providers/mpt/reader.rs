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
    TableId,
    TableRef,
    TableSelector,
};
use fxhash::FxHashMap;
use tracing::{trace, trace_span};
use trie_db::proof::generate_proof;
use trie_db::{DBValue, TrieDBMutBuilder, TrieHash, TrieMut};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::layout::TrieLayoutWrapper;
use super::root::RootTable;
use super::MptStateProof;
use crate::providers::mpt::MptStateTree;
use crate::{
    SimpleHasher,
    StateKey,
    StateRootHash,
    StateTree,
    StateTreeReader,
    VerifyStateTreeError,
};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const ROOT_TABLE_NAME: &str = "%state_tree_root";

pub(crate) type SharedNodesTableRef<'a, B, S, H> =
    Arc<Mutex<TableRef<'a, <SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue, B, S>>>;

pub(crate) type SharedRootTable<'a, B, S> = Arc<Mutex<RootTable<'a, B, S>>>;

trait CloneableStorageBackend: StorageBackend + Clone {}

pub struct MptStateTreeReader<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    // TODO(snormore): Can/should we remove this if it's not used, or should we use it for some of
    // the methods?
    db: Atomo<QueryPerm, B, S>,
    _hasher: PhantomData<H>,
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> Clone for MptStateTreeReader<B, S, H> {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            _hasher: PhantomData,
        }
    }
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MptStateTreeReader<B, S, H>
where
    // TODO(snormore): Can we remove these bounds?
    B: StorageBackend + Send + Sync + Clone,
    S: SerdeBackend + Send + Sync + Clone,
    H: SimpleHasher + Send + Sync + Clone,
{
    pub fn new(db: Atomo<QueryPerm, B, S>) -> Self {
        Self {
            db,
            _hasher: PhantomData,
        }
    }

    /// Get the state root hash of the state tree from the root table if it exists, or compute it
    /// from the state tree nodes table if it does not, and save it to the root table, before
    /// returning it.
    fn state_root(
        &self,
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
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> StateTreeReader<B, S, H>
    for MptStateTreeReader<B, S, H>
where
    // TODO(snormore): Can we remove these bounds?
    B: StorageBackend + Send + Sync + Clone,
    S: SerdeBackend + Send + Sync + Clone,
    H: SimpleHasher + Send + Sync + Clone,
{
    type Proof = MptStateProof;

    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_root(&self, ctx: &TableSelector<B, S>) -> Result<StateRootHash> {
        let span = trace_span!("get_state_root");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        self.state_root(nodes_table, root_table)
    }

    /// Get an existence proof for the given key hash, if it is present in the state tree, or
    /// non-existence proof if it is not present.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_proof(
        &self,
        ctx: &TableSelector<B, S>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof> {
        let span = trace_span!("get_state_proof");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        let state_root: <SimpleHasherWrapper<H> as hash_db::Hasher>::Out =
            self.state_root(nodes_table.clone(), root_table)?.into();
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

    /// Verify that the state in the given atomo database instance, when used to build a new,
    /// temporary state tree from scratch, matches the stored state tree root hash.
    fn verify_state_tree_unsafe(&self, db: &mut Atomo<QueryPerm, B, S>) -> Result<()> {
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
        type TmpTree<S, H> = MptStateTree<InMemoryStorage, S, H>;
        let tmp_tree = TmpTree::<S, H>::new();
        let mut tmp_db =
            TmpTree::<S, H>::register_tables(AtomoBuilder::new(InMemoryStorage::default()))
                .build()?;

        // Apply the batch to the temporary state tree.
        tmp_db.run(|ctx| tmp_tree.update_state_tree(ctx, batch))?;

        // Get and return the state root hash from the temporary state tree.
        let tmp_state_root = tmp_db
            .query()
            .run(|ctx| tmp_tree.reader(tmp_db.query()).get_state_root(ctx))?;

        // Check that the state root hash matches the stored state root hash.
        let stored_state_root = db.query().run(|ctx| self.get_state_root(ctx))?;
        ensure!(
            tmp_state_root == stored_state_root,
            VerifyStateTreeError::StateRootMismatch(stored_state_root, tmp_state_root)
        );

        Ok(())
    }

    // TODO(snormore): Can we do this without mut self?
    fn is_empty_state_tree_unsafe(&self, db: &mut Atomo<QueryPerm, B, S>) -> Result<bool> {
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
