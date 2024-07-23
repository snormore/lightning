use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use anyhow::Result;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::storage::{LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};

use crate::types::{SerializedNodeKey, SerializedNodeValue};

pub struct JmtTreeReader<'a, B: StorageBackend, S: SerdeBackend> {
    tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    values: Arc<RwLock<HashMap<KeyHash, OwnedValue>>>,
}

impl<'a, B: StorageBackend, S: SerdeBackend> JmtTreeReader<'a, B, S> {
    pub fn new(tree_table: &'a TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>) -> Self {
        Self {
            tree_table,
            values: Default::default(),
        }
    }

    pub fn cache(&self, key_hash: KeyHash, value: OwnedValue) {
        let mut values = self.values.write().unwrap();
        values.insert(key_hash, value);
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend> TreeReader for JmtTreeReader<'a, B, S>
where
    B: StorageBackend,
    S: SerdeBackend,
{
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let key = S::serialize(node_key);
        let value = self.tree_table.get(key);
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
        let cache = self.values.read().unwrap();
        let value = cache.get(&key_hash);

        Ok(value.cloned())
    }
}
