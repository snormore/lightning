use std::marker::PhantomData;

use atomo::batch::Operation;
use atomo::{Atomo, SerdeBackend, StorageBackend, TableId, UpdatePerm};
use fxhash::FxHashMap;
use jmt::SimpleHasher;

use crate::jmt::JmtTreeReader;
use crate::types::{SerializedNodeKey, SerializedNodeValue, TableKey};
use crate::{StateTreeReader, StateTreeTableSelector};

// TODO(snormore): This is leaking `jmt::SimpleHasher`.
pub struct StateTreeWriter<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher> {
    inner: Atomo<UpdatePerm, B, S>,
    tree_table_name: String,
    table_name_by_id: FxHashMap<TableId, String>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeWriter<B, S, KH, VH>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    pub fn new(
        inner: Atomo<UpdatePerm, B, S>,
        tree_table_name: String,
        table_id_by_name: FxHashMap<String, TableId>,
    ) -> Self {
        let table_name_by_id = table_id_by_name
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<FxHashMap<TableId, String>>();
        Self {
            inner,
            tree_table_name,
            table_name_by_id,
            _phantom: PhantomData,
        }
    }

    /// Run an update on the data.
    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut StateTreeTableSelector<B, S, KH, VH>) -> R,
    {
        let tree_table_name = self.tree_table_name.clone();
        self.inner.run(|ctx| {
            let mut tree_table =
                ctx.get_table::<SerializedNodeKey, SerializedNodeValue>(tree_table_name);
            let mut ctx = StateTreeTableSelector::new(ctx, &tree_table);
            let res = mutation(&mut ctx);
            // Self::apply_state_tree_changes(
            //     ctx.current_changes(),
            //     &tree_table,
            //     &self.table_name_by_id,
            // );
            let batch = ctx.current_changes();
            let table_name_by_id = &self.table_name_by_id;

            {
                let reader = JmtTreeReader::new(&tree_table);
                let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

                // Iterate over the changes and build the tree value set.
                let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> =
                    Default::default();
                for (table_id, changes) in batch.into_raw().iter().enumerate() {
                    // println!("table {:?} changes {:?}", table_id, changes);
                    let table_id: TableId = table_id.try_into().unwrap();
                    for (key, operation) in changes.iter() {
                        let table_key = TableKey {
                            // TODO(snormore): Fix this unwrap.
                            table: table_name_by_id.get(&table_id).unwrap().to_string(),
                            key: key.to_vec(),
                        };
                        let key_hash = table_key.hash::<S, KH>();

                        // println!("table key {:?} key_hash {:?}", table_key, key_hash);

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

                // Apply the value set to the tree, and get the tree batch that we can convert to
                // atomo storage batches.
                let (_new_root_hash, _update_proof, tree_batch) =
                    tree.put_value_set_with_proof(value_set.clone(), 0).unwrap();

                // Stale nodes are converted to remove operations.
                for stale_node in tree_batch.stale_node_index_batch {
                    let key = S::serialize(&stale_node.node_key);
                    tree_table.remove(key);
                }

                // New nodes are converted to insert operations.
                for (node_key, node) in tree_batch.node_batch.nodes() {
                    let key = S::serialize(node_key);
                    let value = S::serialize(node);
                    tree_table.insert(key, value);
                }
            }

            res
        })
    }

    /// Build and return a query reader for the data.
    pub fn query(&self) -> StateTreeReader<B, S, KH, VH> {
        StateTreeReader::new(self.inner.query(), self.tree_table_name.clone())
    }

    /// Return the internal storage backend.
    pub fn get_storage_backend_unsafe(&mut self) -> &B {
        self.inner.get_storage_backend_unsafe()
    }

    // fn apply_state_tree_changes<'a>(
    //     batch: VerticalBatch,
    //     tree_table: &'a mut TableRef<'a, SerializedNodeKey, SerializedNodeValue, B, S>,
    //     table_name_by_id: &FxHashMap<TableId, String>,
    // ) {
    //     let reader = JmtTreeReader::new(tree_table);
    //     let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

    //     // Iterate over the changes and build the tree value set.
    //     let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
    //     for (table_id, changes) in batch.into_raw().iter().enumerate() {
    //         // println!("table {:?} changes {:?}", table_id, changes);
    //         let table_id: TableId = table_id.try_into().unwrap();
    //         for (key, operation) in changes.iter() {
    //             let table_key = TableKey {
    //                 // TODO(snormore): Fix this unwrap.
    //                 table: table_name_by_id.get(&table_id).unwrap().to_string(),
    //                 key: key.to_vec(),
    //             };
    //             let key_hash = table_key.hash::<KH>();

    //             // println!("table key {:?} key_hash {:?}", table_key, key_hash);

    //             match operation {
    //                 Operation::Remove => {
    //                     value_set.push((key_hash, None));
    //                 },
    //                 Operation::Insert(value) => {
    //                     value_set.push((key_hash, Some(value.to_vec())));
    //                 },
    //             }
    //         }
    //     }

    //     // Apply the value set to the tree, and get the tree batch that we can convert to atomo
    //     // storage batches.
    //     let (_new_root_hash, _update_proof, tree_batch) =
    //         tree.put_value_set_with_proof(value_set.clone(), 0).unwrap();

    //     // Stale nodes are converted to remove operations.
    //     for stale_node in tree_batch.stale_node_index_batch {
    //         // println!("stale node {:?}", stale_node);
    //         tree_table.remove(&to_vec(&stale_node.node_key).unwrap());
    //     }

    //     // New nodes are converted to insert operations.
    //     for (node_key, node) in tree_batch.node_batch.nodes() {
    //         // println!("node key {:?} node {:?}", node_key, node);
    //         tree_table.insert(to_vec(node_key).unwrap(), to_vec(node).unwrap());
    //     }
    // }
}

// #[cfg(test)]
// mod tests {
//     use atomo::{InMemoryStorage, StorageBackendConstructor};

//     use super::*;
//     use crate::keccak::KeccakHasher;

//     #[test]
//     fn test_commit() {
//         type KeyHasher = blake3::Hasher;
//         type ValueHasher = KeccakHasher;

//         let mut storage = InMemoryStorage::default();
//         let data_table_id = storage.open_table("data".to_string());
//         let tree_table_id = storage.open_table("tree".to_string());
//         let storage = Arc::new(storage);

//         let writer =
//             StateTreeWriter::<_, KeyHasher, ValueHasher>::new(storage.clone(),
// "tree".to_string());

//         let mut batch = VerticalBatch::new(2);
//         let insert_count = 10;
//         for i in 1..=insert_count {
//             batch.insert(
//                 data_table_id,
//                 format!("key{i}").as_bytes().to_vec().into(),
//                 Operation::Insert(format!("value{i}").as_bytes().to_vec().into()),
//             );
//         }

//         // writer.commit(batch);

//         let keys = storage.keys(data_table_id);
//         assert_eq!(keys.len(), insert_count);

//         let keys = storage.keys(tree_table_id);
//         assert_eq!(keys.len(), 12);
//     }
// }
