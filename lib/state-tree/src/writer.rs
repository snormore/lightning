use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use atomo::batch::{BatchHashMap, BoxedVec, Operation, VerticalBatch};
use atomo::{StorageBackend, TableId};
use borsh::to_vec;
use jmt::SimpleHasher;

use crate::jmt::JmtTreeReader;
use crate::types::TableKey;

// TODO(snormore): This is leaking `jmt::SimpleHasher`.
pub struct StateTreeWriter<S: StorageBackend, KH: SimpleHasher, VH: SimpleHasher> {
    storage: Arc<S>,
    nodes_table_index: TableId,
    table_id_by_name: HashMap<String, TableId>,
    table_name_by_id: HashMap<TableId, String>,
    _kv_hashers: PhantomData<(KH, VH)>,
}

impl<S: StorageBackend, KH: SimpleHasher, VH: SimpleHasher> StateTreeWriter<S, KH, VH>
where
    S: StorageBackend + Send + Sync,
{
    pub fn new(
        storage: Arc<S>,
        nodes_table_index: TableId,
        table_id_by_name: HashMap<String, TableId>,
    ) -> Self {
        Self {
            storage,
            table_id_by_name: table_id_by_name.clone(),
            table_name_by_id: table_id_by_name
                .clone()
                .into_iter()
                .map(|(k, v)| (v, k))
                .collect::<HashMap<TableId, String>>(),
            nodes_table_index,
            _kv_hashers: PhantomData,
        }
    }

    fn extend_commit_batch(&self, batch: VerticalBatch) -> VerticalBatch {
        let reader = JmtTreeReader::new(
            &*self.storage,
            self.nodes_table_index,
            &self.table_id_by_name,
        );
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        // Iterate over the changes and build the tree value set.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        for (table_id, changes) in batch.clone().into_raw().iter().enumerate() {
            let table_id: TableId = table_id.try_into().unwrap();
            for (key, operation) in changes.iter() {
                let table_key = TableKey {
                    table: self.table_name_by_id.get(&table_id).unwrap().to_string(),
                    key: key.to_vec(),
                };
                let key_hash = table_key.hash::<KH>();

                match operation {
                    Operation::Remove => {
                        value_set.push((key_hash, None));
                    },
                    Operation::Insert(value) => {
                        value_set.push((key_hash, Some(value.to_vec())));
                    },
                }
            }
        }

        let mut nodes_storage_batch = BatchHashMap::default();

        // Apply the value set to the tree, and get the tree batch that we can convert to atomo
        // storage batches.
        let (_new_root_hash, _update_proof, tree_batch) =
            tree.put_value_set_with_proof(value_set.clone(), 0).unwrap();

        // Stale nodes are converted to remove operations.
        for stale_node in tree_batch.stale_node_index_batch {
            nodes_storage_batch.insert(
                to_vec(&stale_node.node_key).unwrap().into(),
                Operation::Remove,
            );
        }

        // New nodes are converted to insert operations.
        for (node_key, node) in tree_batch.node_batch.nodes() {
            nodes_storage_batch.insert(
                to_vec(node_key).unwrap().into(),
                Operation::Insert(to_vec(node).unwrap().into()),
            );
        }

        batch.extend(self.nodes_table_index, nodes_storage_batch)
    }
}

impl<S: StorageBackend, KH: SimpleHasher, VH: SimpleHasher> atomo::StorageBackend
    for StateTreeWriter<S, KH, VH>
where
    S: StorageBackend + Send + Sync,
{
    fn commit(&self, batch: VerticalBatch) {
        // TODO(snormore): Assumption: The tree table is always the last table opened.
        // TODO(snormore): Assumption: The given batch does not include any changes to the tree
        // table.
        let batch = self.extend_commit_batch(batch);
        self.storage.commit(batch);
    }

    fn keys(&self, tid: TableId) -> Vec<BoxedVec> {
        self.storage.keys(tid)
    }

    fn get(&self, tid: TableId, key: &[u8]) -> Option<Vec<u8>> {
        self.storage.get(tid, key)
    }

    fn contains(&self, tid: TableId, key: &[u8]) -> bool {
        self.storage.contains(tid, key)
    }
}
#[cfg(test)]
mod tests {
    use atomo::{InMemoryStorage, StorageBackendConstructor};

    use super::*;
    use crate::keccak::KeccakHasher;

    #[test]
    fn test_commit() {
        type KeyHasher = blake3::Hasher;
        type ValueHasher = KeccakHasher;

        let mut storage = InMemoryStorage::default();
        let data_table_id = storage.open_table("data".to_string());
        let tree_table_id = storage.open_table("tree".to_string());
        let storage = Arc::new(storage);

        let table_id_by_name: HashMap<String, TableId> = vec![
            ("data".to_string(), data_table_id),
            ("tree".to_string(), tree_table_id),
        ]
        .into_iter()
        .collect();

        let writer = StateTreeWriter::<_, KeyHasher, ValueHasher>::new(
            storage.clone(),
            tree_table_id,
            table_id_by_name,
        );

        let mut batch = VerticalBatch::new(2);
        let insert_count = 10;
        for i in 1..=insert_count {
            batch.insert(
                data_table_id,
                format!("key{i}").as_bytes().to_vec().into(),
                Operation::Insert(format!("value{i}").as_bytes().to_vec().into()),
            );
        }

        writer.commit(batch);

        let keys = storage.keys(data_table_id);
        assert_eq!(keys.len(), insert_count);

        let keys = storage.keys(tree_table_id);
        assert_eq!(keys.len(), 12);
    }
}
