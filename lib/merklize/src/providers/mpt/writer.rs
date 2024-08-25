use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use atomo::batch::{BoxedVec, Operation, VerticalBatch};
use atomo::{
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableId,
    TableSelector,
    UpdatePerm,
};
use fxhash::FxHashMap;
use tracing::{trace, trace_span};
use trie_db::{TrieDBMutBuilder, TrieMut};

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
use crate::{SimpleHasher, StateKey, StateRootHash, StateTree, StateTreeWriter};

pub struct MptStateTreeWriter<T: StateTree> {
    _tree: PhantomData<T>,
}

impl<T: StateTree> MptStateTreeWriter<T>
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
    // TODO(snormore): Remove/consolidate this duplicate method from `MptStateTreeReader`.
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

impl<T: StateTree> StateTreeWriter<T> for MptStateTreeWriter<T>
where
    // Send + Sync bounds required by
    // triedb/hashdb.
    T::StorageBuilder: StorageBackendConstructor + Send + Sync,
    <T::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync,
    T::Serde: SerdeBackend + Send + Sync,
    T::Hasher: SimpleHasher + Send + Sync,
{
    /// Apply the state tree changes based on the state changes in the atomo batch. This will update
    /// the state tree to reflect the changes in the atomo batch.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn update_state_tree<I>(
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
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
        let mut state_root: <SimpleHasherWrapper<T::Hasher> as hash_db::Hasher>::Out =
            Self::state_root(nodes_table.clone(), root_table.clone())?.into();

        // Initialize a `TrieDBMutBuilder` to update the state tree.
        let mut adapter = Adapter::<
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
            T::Hasher,
        >::new(nodes_table.clone());
        let mut tree = TrieDBMutBuilder::<TrieLayoutWrapper<T::Hasher>>::from_existing(
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
                let key_hash = state_key.hash::<T::Serde, T::Hasher>();

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
        db: &mut atomo::Atomo<
            UpdatePerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
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
