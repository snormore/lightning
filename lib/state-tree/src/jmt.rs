use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use atomo::{StorageBackend, TableId};
use borsh::{from_slice, to_vec};
use jmt::storage::{LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};

use crate::types::TableKey;

pub struct JmtTreeReader<'a, S: StorageBackend> {
    pub storage: &'a S,
    pub nodes_table_index: TableId,
    pub keys: Arc<RwLock<HashMap<KeyHash, TableKey>>>,
}

impl<'a, S: StorageBackend> JmtTreeReader<'a, S> {
    pub fn new(storage: &'a S, nodes_table_index: TableId) -> Self {
        Self {
            storage,
            nodes_table_index,
            keys: Default::default(),
        }
    }

    pub fn cache_key(&self, key_hash: KeyHash, table_key: TableKey) {
        let mut keys = self.keys.write().unwrap();
        keys.insert(key_hash, table_key);
    }
}

impl<'a, S: StorageBackend> TreeReader for JmtTreeReader<'a, S>
where
    S: StorageBackend + Send + Sync,
{
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let value = self
            .storage
            .get(self.nodes_table_index, &to_vec(node_key).unwrap());
        match value {
            Some(value) => Ok(Some(from_slice(&value).unwrap())),
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
        let keys_cache = self.keys.read().unwrap();
        let table_key = keys_cache.get(&key_hash);
        match table_key {
            Some(table_key) => {
                let value = self.storage.get(table_key.table, &table_key.key);
                Ok(value)
            },
            None => Ok(None),
        }
    }
}
