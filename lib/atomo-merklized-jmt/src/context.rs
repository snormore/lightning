use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use atomo::batch::Operation;
use atomo::{SerdeBackend, StorageBackend, TableIndex, TableSelector};
use atomo_merklized::{MerklizedContext, SimpleHasher, StateKey, StateRootHash};
use fxhash::FxHashMap;
use jmt::storage::{HasPreimage, LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};
use log::trace;

use crate::hasher::SimpleHasherWrapper;
use crate::strategy::{KEYS_TABLE_NAME, NODES_TABLE_NAME};

type SharedTableRef<'a, K, V, B, S> = Arc<Mutex<atomo::TableRef<'a, K, V, B, S>>>;

pub struct JmtMerklizedContext<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    ctx: &'a TableSelector<B, S>,
    table_name_by_id: FxHashMap<TableIndex, String>,
    nodes_table: SharedTableRef<'a, Vec<u8>, Vec<u8>, B, S>,
    keys_table: SharedTableRef<'a, KeyHash, StateKey, B, S>,
    _phantom: PhantomData<H>,
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> JmtMerklizedContext<'a, B, S, H> {
    pub fn new(ctx: &'a TableSelector<B, S>) -> Self {
        let tables = ctx.tables();

        let nodes_table = ctx.get_table(NODES_TABLE_NAME);
        let keys_table = ctx.get_table(KEYS_TABLE_NAME);

        let mut table_id_by_name = FxHashMap::default();
        for (i, table) in tables.iter().enumerate() {
            let table_id: TableIndex = i.try_into().unwrap();
            let table_name = table.name.to_string();
            table_id_by_name.insert(table_name, table_id);
        }

        let table_name_by_id = table_id_by_name
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<FxHashMap<TableIndex, String>>();

        Self {
            ctx,
            table_name_by_id,
            nodes_table: Arc::new(Mutex::new(nodes_table)),
            keys_table: Arc::new(Mutex::new(keys_table)),
            _phantom: PhantomData,
        }
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MerklizedContext<'a, B, S, H>
    for JmtMerklizedContext<'a, B, S, H>
{
    fn get_state_root(&self) -> Result<StateRootHash> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        tree.get_root_hash(0).map(|hash| hash.0.into())
    }

    fn get_state_proof(
        &self,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<(Option<Vec<u8>>, ics23::CommitmentProof)> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<S, H>();
        trace!(key_hash:?, state_key:?; "get_proof");

        let (value, proof) = tree.get_with_ics23_proof(S::serialize(&state_key), 0)?;

        Ok((value, proof))
    }

    fn apply_state_tree_changes(&mut self) -> Result<()> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        // Build a jmt value set (batch) from the atomo batch.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        let batch = self.ctx.batch();
        for (table_id, changes) in batch.into_raw().iter().enumerate() {
            let table_id: TableIndex = table_id.try_into()?;
            let table_name = self
                .table_name_by_id
                .get(&table_id)
                .ok_or(anyhow!("Table with index {} not found", table_id))?
                .as_str();
            for (key, operation) in changes.iter() {
                let state_key = StateKey::new(table_name, key.to_vec());
                let key_hash = jmt::KeyHash(state_key.hash::<S, H>().into());

                match operation {
                    Operation::Remove => {
                        value_set.push((key_hash, None));

                        // Remove it from the keys table.
                        trace!(key_hash:?, state_key:?; "removing key");
                        self.keys_table.lock().unwrap().remove(key_hash);
                    },
                    Operation::Insert(value) => {
                        value_set.push((key_hash, Some(value.to_vec())));

                        // Insert it into the keys table.
                        trace!(key_hash:?, state_key:?; "inserting key");
                        self.keys_table.lock().unwrap().insert(key_hash, state_key);
                    },
                }
            }
        }

        // Apply the jmt value set (batch) to the tree.
        let (_new_root_hash, tree_batch) = tree.put_value_set(value_set.clone(), 0).unwrap();

        // Remove stale nodes.
        for stale_node in tree_batch.stale_node_index_batch {
            let key = S::serialize(&stale_node.node_key);
            self.nodes_table.lock().unwrap().remove(key);
        }

        // Insert new nodes.
        for (node_key, node) in tree_batch.node_batch.nodes() {
            let key = S::serialize(node_key);
            let value = S::serialize(node);
            self.nodes_table.lock().unwrap().insert(key, value);
        }

        Ok(())
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> TreeReader
    for JmtMerklizedContext<'a, B, S, H>
{
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let key = S::serialize(node_key);
        let value = self.nodes_table.lock().unwrap().get(key);
        match value {
            Some(value) => Ok(Some(S::deserialize(&value))),
            None => Ok(None),
        }
    }

    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        // Not currently used.
        unimplemented!()
    }

    fn get_value_option(
        &self,
        _max_version: Version,
        key_hash: KeyHash,
    ) -> Result<Option<OwnedValue>> {
        // TODO(snormore): Keep a cache of these lookups.
        let state_key = { self.keys_table.lock().unwrap().get(key_hash) };
        let value = if let Some(state_key) = state_key {
            self.ctx.get_raw_value(state_key.table, &state_key.key)
        } else {
            None
        };
        trace!(key_hash:?, value:?; "get_value_option");
        Ok(value)
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> HasPreimage
    for JmtMerklizedContext<'a, B, S, H>
{
    /// Gets the preimage of a key hash, if it is present in the tree.
    fn preimage(&self, key_hash: KeyHash) -> Result<Option<Vec<u8>>> {
        // TODO(snormore): Keep a cache of these lookups.
        let state_key = self.keys_table.lock().unwrap().get(key_hash);
        trace!(key_hash:?, state_key:?; "preimage");
        Ok(state_key.map(|key| S::serialize(&key)))
    }
}
