use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use atomo::batch::{BoxedVec, Operation, VerticalBatch};
use atomo::{
    Atomo,
    AtomoBuilder,
    QueryPerm,
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
use trie_db::{DBValue, TrieDBMutBuilder, TrieMut};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::layout::TrieLayoutWrapper;
use super::root::RootTable;
use super::MptStateTreeReader;
use crate::{SimpleHasher, StateKey, StateRootHash, StateTree};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const ROOT_TABLE_NAME: &str = "%state_tree_root";

pub(crate) type SharedNodesTableRef<'a, B, S, H> =
    Arc<Mutex<TableRef<'a, <SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue, B, S>>>;

pub(crate) type SharedRootTable<'a, B, S> = Arc<Mutex<RootTable<'a, B, S>>>;

pub struct MptStateTree<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> {
    _storage: PhantomData<B>,
    _serde: PhantomData<S>,
    _hasher: PhantomData<H>,
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> MptStateTree<B, S, H>
where
    // Send + Sync bounds required by triedb/hashdb.
    B: StorageBackendConstructor + Send + Sync,
    <B as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _storage: PhantomData,
            _serde: PhantomData,
            _hasher: PhantomData,
        }
    }

    /// Get the state root hash of the state tree from the root table if it exists, or compute it
    /// from the state tree nodes table if it does not, and save it to the root table, before
    /// returning it.
    // TODO(snormore): Can we DRY this up and use the one in the reader?
    fn state_root(
        nodes_table: SharedNodesTableRef<B::Storage, S, H>,
        root_table: SharedRootTable<B::Storage, S>,
    ) -> Result<StateRootHash> {
        let root = { root_table.lock().unwrap().get() };
        if let Some(root) = root {
            Ok(root)
        } else {
            let mut root: <SimpleHasherWrapper<H> as hash_db::Hasher>::Out =
                StateRootHash::default().into();
            let mut adapter = Adapter::<<B as StorageBackendConstructor>::Storage, S, H>::new(
                nodes_table.clone(),
            );
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

impl<D, S, H> StateTree for MptStateTree<D, S, H>
where
    // Send + Sync bounds required by triedb/hashdb.
    // Clone bounds required by
    // SyncQueryRunnerInterface.
    D: DatabaseRunContext + Send + Sync,
    <D as DatabaseRunContext>::Storage: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync + Clone,
    H: SimpleHasher + Send + Sync + Clone,
{
    type DatabaseBuilder = D;
    type Serde = S;
    type Hasher = H;

    type Reader = MptStateTreeReader<D, S, H>;

    fn new() -> Self {
        Self::new()
    }

    // TODO(snormore): Can we do this without passing in a db? Does the tree need a db on the
    // instance?
    fn reader(&self, db: Atomo<QueryPerm, B::Storage, S>) -> Self::Reader {
        MptStateTreeReader::new(db.query())
    }

    fn register_tables(
        builder: AtomoBuilder<Self::StorageBuilder, Self::Serde>,
    ) -> AtomoBuilder<Self::StorageBuilder, Self::Serde> {
        builder
            .with_table::<<SimpleHasherWrapper<Self::Hasher> as hash_db::Hasher>::Out, DBValue>(
                NODES_TABLE_NAME,
            )
            .with_table::<u8, StateRootHash>(ROOT_TABLE_NAME)
    }

    /// Apply the state tree changes based on the state changes in the atomo batch. This will update
    /// the state tree to reflect the changes in the atomo batch.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn update_state_tree<I>(
        &self,
        ctx: &TableSelector<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
        batch: HashMap<String, I>,
    ) -> Result<()>
    where
        I: Iterator<Item = (BoxedVec, Operation)>,
    {
        let span = trace_span!("update_state_tree");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let root_table = Arc::new(Mutex::new(RootTable::new(ctx)));

        // Get the current state root hash.
        let mut state_root: <SimpleHasherWrapper<Self::Hasher> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table.clone())?.into();

        // Initialize a `TrieDBMutBuilder` to update the state tree.
        let mut adapter = Adapter::<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
            Self::Hasher,
        >::new(nodes_table.clone());
        let mut tree = TrieDBMutBuilder::<TrieLayoutWrapper<Self::Hasher>>::from_existing(
            &mut adapter,
            &mut state_root,
        )
        .build();

        // Apply the changes in the batch to the state tree.
        for (table, changes) in batch {
            if table == NODES_TABLE_NAME || table == ROOT_TABLE_NAME {
                continue;
            }

            for (key, operation) in changes {
                let state_key = StateKey::new(&table, key.to_vec());
                let key_hash = state_key.hash::<Self::Serde, Self::Hasher>();

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

    /// Clear the state tree by removing all nodes and keys from the atomo database.
    fn clear_state_tree_unsafe(
        &self,
        db: &mut Atomo<
            UpdatePerm,
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("clear_state_tree");
        let _enter = span.enter();

        let tables = db.tables();
        let table_id_by_name = tables
            .iter()
            .enumerate()
            .map(|(tid, table)| (table.clone(), tid as TableId))
            .collect::<FxHashMap<_, _>>();

        let nodes_table_id = *table_id_by_name.get(NODES_TABLE_NAME).unwrap();
        let root_table_id = *table_id_by_name.get(ROOT_TABLE_NAME).unwrap();

        let mut batch = VerticalBatch::new(tables.len());
        let storage = db.get_storage_backend_unsafe();

        // Remove nodes table entries.
        let nodes_table_batch = batch.get_mut(nodes_table_id as usize);
        for key in storage.keys(nodes_table_id) {
            nodes_table_batch.insert(key, Operation::Remove);
        }

        // Remove root table entries.
        let root_table_batch = batch.get_mut(root_table_id as usize);
        for key in storage.keys(root_table_id) {
            root_table_batch.insert(key, Operation::Remove);
        }

        // Commit the batch.
        storage.commit(batch);

        Ok(())
    }
}
