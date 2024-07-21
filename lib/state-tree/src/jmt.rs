use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use atomo::{SerdeBackend, StorageBackend, TableSelector};
use borsh::{from_slice, to_vec};
use jmt::storage::{LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};

use crate::types::{SerializedNodeKey, SerializedNodeValue, TableKey};

pub struct JmtTreeReader<'a, B: StorageBackend, S: SerdeBackend> {
    ctx: &'a TableSelector<B, S>,
    tree_table_name: String,
    keys: Arc<RwLock<HashMap<KeyHash, TableKey>>>,
}

impl<'a, B: StorageBackend, S: SerdeBackend> JmtTreeReader<'a, B, S> {
    pub fn new(ctx: &'a TableSelector<B, S>, tree_table_name: String) -> Self {
        Self {
            ctx,
            tree_table_name,
            keys: Default::default(),
        }
    }

    pub fn cache_key(&self, key_hash: KeyHash, table_key: TableKey) {
        let mut keys = self.keys.write().unwrap();
        keys.insert(key_hash, table_key);
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend> TreeReader for JmtTreeReader<'a, B, S>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let tree_table = self
            .ctx
            .get_table::<SerializedNodeKey, SerializedNodeValue>(self.tree_table_name.clone());
        let value = tree_table.get(to_vec(node_key).unwrap());
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
                let table = self
                    .ctx
                    // TODO(snormore): Fix the types being passed to get_table here; it should be
                    // based on which table is being accessed.
                    .get_table::<SerializedNodeKey, SerializedNodeValue>(table_key.table.clone());
                Ok(table.get(table_key.key.clone()))
            },
            None => Ok(None),
        }
    }
}
