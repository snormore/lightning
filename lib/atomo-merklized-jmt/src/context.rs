use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use atomo::batch::Operation;
use atomo::{SerdeBackend, StorageBackend, TableIndex, TableSelector};
use atomo_merklized::{MerklizedContext, SimpleHasher, StateKey, StateRootHash, StateTable};
use fxhash::FxHashMap;
use jmt::storage::{HasPreimage, LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};
use log::trace;

use crate::hasher::SimpleHasherWrapper;

type SharedTableRef<'a, K, V, B, S> = Arc<Mutex<atomo::TableRef<'a, K, V, B, S>>>;

pub struct JmtMerklizedContext<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    ctx: &'a TableSelector<B, S>,
    table_name_by_id: FxHashMap<TableIndex, String>,
    nodes_table: SharedTableRef<'a, Vec<u8>, Vec<u8>, B, S>,
    keys_table: SharedTableRef<'a, KeyHash, StateKey, B, S>,
    values_table: SharedTableRef<'a, KeyHash, Vec<u8>, B, S>,
    _phantom: PhantomData<H>,
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> JmtMerklizedContext<'a, B, S, H> {
    pub fn new(ctx: &'a TableSelector<B, S>) -> Self {
        let tables = ctx.tables();

        // TODO(snormore): Pass in the table name prefix as a parameter.
        let nodes_table = ctx.get_table("%state_tree_nodes");
        let keys_table = ctx.get_table("%state_tree_keys");
        let values_table = ctx.get_table("%state_tree_values");

        let mut table_id_by_name = FxHashMap::default();
        for (i, table) in tables.iter().enumerate() {
            let table_id: TableIndex = i.try_into().unwrap();
            table_id_by_name.insert(table._name.to_string(), table_id);
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
            values_table: Arc::new(Mutex::new(values_table)),
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
        key: Vec<u8>,
    ) -> Result<(Option<Vec<u8>>, ics23::CommitmentProof)> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        // Get the value and proof from the tree.
        // TODO(snormore): We don't need StateTable, just build a StateKey directly.
        let state_key = StateTable::new(table).key(key);
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
                let state_key = StateKey::new(table_name.to_string(), key.to_vec());
                let key_hash = jmt::KeyHash(state_key.hash::<S, H>().into());

                match operation {
                    Operation::Remove => {
                        value_set.push((key_hash, None));

                        // Remove it from the keys table.
                        trace!(key_hash:?, state_key:?; "removing key");
                        self.keys_table.lock().unwrap().remove(key_hash);

                        // Remove it from the values table.
                        trace!(key_hash:?; "removing value");
                        self.values_table.lock().unwrap().remove(key_hash);
                    },
                    Operation::Insert(value) => {
                        value_set.push((key_hash, Some(value.to_vec())));

                        // Insert it into the keys table.
                        trace!(key_hash:?, state_key:?; "inserting key");
                        self.keys_table.lock().unwrap().insert(key_hash, state_key);

                        // Insert it into the values table.
                        trace!(key_hash:?, value:?; "inserting value");
                        self.values_table
                            .lock()
                            .unwrap()
                            .insert(key_hash, value.to_vec());
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
        // let state_key = { self.keys_table.lock().unwrap().get(key_hash) };
        // if let Some(state_key) = state_key {
        //     // TODO(snormore): Need a way to get the table K,V metadata for this.
        //     // let table = self.ctx.get_table(state_key.table);
        //     trace!(key_hash:?, state_key:?; "get_value_option");
        //     todo!("get_value_option not implemented yet")
        // } else {
        //     return Ok(None);
        // }
        // TODO(snormore): This values table is very wasteful, and we should lookup the value from
        // the state tables directly instead.
        let value = self.values_table.lock().unwrap().get(key_hash);
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
