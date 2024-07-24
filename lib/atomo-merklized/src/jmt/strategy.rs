use std::marker::PhantomData;

use anyhow::Result;
use atomo::batch::{Operation, VerticalBatch};
use atomo::{SerdeBackend, StorageBackend, TableId, TableRef};
use fxhash::FxHashMap;
use jmt::proof::SparseMerkleProof;
use jmt::{KeyHash, SimpleHasher};

use super::JmtTreeReader;
use crate::{
    MerklizedStrategy,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateKey,
    StateRootHash,
};

pub struct JmtMerklizedStrategy<
    'a,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
> {
    tree_table: TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
    table_name_by_id: FxHashMap<TableId, String>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<'a, B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    JmtMerklizedStrategy<'a, B, S, KH, VH>
{
    pub fn new(
        tree_table: TableRef<'a, SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table_name_by_id: FxHashMap<TableId, String>,
    ) -> Self {
        Self {
            tree_table,
            table_name_by_id,
            _phantom: PhantomData,
        }
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    MerklizedStrategy<B, S, KH, VH> for JmtMerklizedStrategy<'a, B, S, KH, VH>
{
    // fn build(tree_table: TableRef<SerializedNodeKey, SerializedTreeNodeValue, B, S>) -> &Self {
    //     &JmtMerklizedStrategy::<'a, B, S, KH, VH>::new(tree_table)
    // }

    fn tree_table(&self) -> &TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S> {
        &self.tree_table
    }

    fn get_root_hash(&self) -> Result<StateRootHash> {
        let reader = JmtTreeReader::new(&self.tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        tree.get_root_hash(0).map(|hash| hash.0.into())
    }

    fn get_with_proof(
        &self,
        table: String,
        key: Vec<u8>,
        // TODO(snormore): Should not have to pass in the value here.
        value: Option<Vec<u8>>,
    ) -> Result<(Option<Vec<u8>>, SparseMerkleProof<VH>)> {
        let reader = JmtTreeReader::new(&self.tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        // Cache the value in the reader so it can be used when `get_with_proof` is called next,
        // which calls `get_value_option`, which needs the value mapping.
        let key = StateKey { table, key };
        let key_hash = KeyHash(key.hash::<S, KH>().into());
        if let Some(value) = value {
            reader.cache(key_hash, S::serialize(&value));
        }
        // TODO(snormore): Fix this unwrap.
        let (value, proof) = tree.get_with_proof(key_hash, 0).unwrap();
        let value = value.map(|value| S::deserialize(&value));

        Ok((value, proof))
    }

    fn apply_changes(&mut self, batch: VerticalBatch) -> Result<()> {
        let reader = JmtTreeReader::new(&self.tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        // Iterate over the changes and build the tree value set.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        for (table_id, changes) in batch.into_raw().iter().enumerate() {
            // println!("table {:?} changes {:?}", table_id, changes);
            let table_id: TableId = table_id.try_into().unwrap();
            for (key, operation) in changes.iter() {
                let table_key = StateKey {
                    // TODO(snormore): Fix this unwrap.
                    table: self.table_name_by_id.get(&table_id).unwrap().to_string(),
                    key: key.to_vec(),
                };
                let key_hash = KeyHash(table_key.hash::<S, KH>().into());

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
            // TODO(snormore): This is unecessarily/redundantly serializing. Instead, we should pass
            // in a key type that implements Serialize, because internally it serializes the key
            // again.
            self.tree_table.remove(key);
        }

        // New nodes are converted to insert operations.
        for (node_key, node) in tree_batch.node_batch.nodes() {
            // TODO(snormore): This is unecessarily/redundantly serializing. Instead, we should pass
            // in a key/value type that implements Serialize, because internally it serializes the
            // key/value again.
            let key = S::serialize(node_key);
            let value = S::serialize(node);
            self.tree_table.insert(key, value);
        }

        Ok(())
    }
}
