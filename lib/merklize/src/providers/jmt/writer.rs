use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use atomo::batch::{BoxedVec, Operation, VerticalBatch};
use atomo::{Atomo, StorageBackend, StorageBackendConstructor, TableId, TableSelector, UpdatePerm};
use fxhash::FxHashMap;
use jmt::storage::{Node, NodeKey, TreeReader};
use jmt::KeyHash;
use tracing::{trace, trace_span};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::tree::{KEYS_TABLE_NAME, NODES_TABLE_NAME, TREE_VERSION};
use crate::{StateKey, StateTree, StateTreeWriter};

pub struct JmtStateTreeWriter<T: StateTree> {
    db: Atomo<
        UpdatePerm,
        <<T as StateTree>::StorageBuilder as StorageBackendConstructor>::Storage,
        <T as StateTree>::Serde,
    >,

    _tree: PhantomData<T>,
}

impl<T: StateTree> StateTreeWriter<T> for JmtStateTreeWriter<T> {
    fn new(
        db: Atomo<
            UpdatePerm,
            <<T as StateTree>::StorageBuilder as StorageBackendConstructor>::Storage,
            <T as StateTree>::Serde,
        >,
    ) -> Self {
        Self {
            db,

            _tree: PhantomData,
        }
    }

    fn build(
        self,
        builder: atomo::AtomoBuilder<<T as StateTree>::StorageBuilder, <T as StateTree>::Serde>,
    ) -> Result<Self> {
        let db = builder
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .map_err(|e| anyhow!("{:?}", e))?;

        Ok(Self::new(db))
    }

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
        let keys_table = Arc::new(Mutex::new(ctx.get_table(KEYS_TABLE_NAME)));

        let adapter = Adapter::new(ctx, nodes_table.clone(), keys_table.clone());
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<T::Hasher>>::new(&adapter);

        // Build a jmt value set (batch) from the atomo batch.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        {
            let span = trace_span!("build_value_set");
            let _enter = span.enter();

            for (table, changes) in batch {
                if table == NODES_TABLE_NAME || table == KEYS_TABLE_NAME {
                    continue;
                }

                for (key, operation) in changes {
                    let state_key = StateKey::new(&table, key.to_vec());
                    let key_hash = jmt::KeyHash(state_key.hash::<T::Serde, T::Hasher>().into());

                    match operation {
                        Operation::Remove => {
                            value_set.push((key_hash, None));

                            // Remove it from the keys table.
                            trace!(?key_hash, ?state_key, "removing key");
                            keys_table.lock().unwrap().remove(key_hash);
                        },
                        Operation::Insert(value) => {
                            let existing_value =
                                adapter.get_value_option(TREE_VERSION, key_hash)?;
                            if let Some(existing_value) = existing_value {
                                if existing_value == value.to_vec() {
                                    // If the key already exists with the same value, we
                                    // shouldn't insert it again. The storage backend deals with
                                    // this, but we should avoid inserting it into the tree
                                    // again. The `jmt` crate does not handle duplicate keys at
                                    // the moment, so we need do it here.
                                    break;
                                }
                            }

                            value_set.push((key_hash, Some(value.to_vec())));

                            // Insert it into the keys table.
                            trace!(?key_hash, ?state_key, "inserting key");
                            keys_table
                                .lock()
                                .unwrap()
                                .insert(key_hash, state_key.clone());
                        },
                    }
                }
            }
        }

        // Apply the jmt value set (batch) to the tree.
        let tree_batch = {
            let span = trace_span!("put_value_set");
            let _enter = span.enter();

            let (_new_root_hash, tree_batch) =
                tree.put_value_set(value_set.clone(), TREE_VERSION).unwrap();
            tree_batch
        };

        // Remove stale nodes.
        {
            let span = trace_span!("remove_stale_nodes");
            let _enter = span.enter();

            for stale_node in tree_batch.stale_node_index_batch {
                trace!(?stale_node, "removing stale node");

                nodes_table.lock().unwrap().remove(stale_node.node_key);
            }
        }

        // Insert new nodes.
        {
            let span = trace_span!("insert_new_nodes");
            let _enter = span.enter();

            for (node_key, node) in tree_batch.node_batch.nodes() {
                trace!(?node_key, ?node, "inserting new node");

                let mut nodes_table = nodes_table.lock().unwrap();

                if node_key.nibble_path().is_empty() {
                    // If the nibble path is empty, it's a root node and we should also insert it to
                    // the previous version, since `jmt` crate expects it, while our usage of `jmt`
                    // is with a single version.
                    let node_key =
                        NodeKey::new(node_key.version() - 1, node_key.nibble_path().clone());
                    nodes_table.insert(node_key, node);
                }

                nodes_table.insert(node_key, node);
            }
        }

        Ok(())
    }

    /// Clear the state tree by removing all nodes and keys from the atomo database.
    fn clear_state_tree_unsafe(
        db: &mut Atomo<
            atomo::UpdatePerm,
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
        let keys_table_id = *table_id_by_name.get(KEYS_TABLE_NAME).unwrap();

        let mut batch = VerticalBatch::new(tables.len());
        let storage = db.get_storage_backend_unsafe();

        // Remove nodes table entries.
        let nodes_table_batch = batch.get_mut(nodes_table_id as usize);
        for key in storage.keys(nodes_table_id) {
            nodes_table_batch.insert(key, Operation::Remove);
        }

        // Remove root table entries.
        let keys_table_batch = batch.get_mut(keys_table_id as usize);
        for key in storage.keys(keys_table_id) {
            keys_table_batch.insert(key, Operation::Remove);
        }

        // Commit the batch.
        storage.commit(batch);

        Ok(())
    }
}
