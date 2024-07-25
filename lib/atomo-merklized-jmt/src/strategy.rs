use std::marker::PhantomData;

use anyhow::Result;
use atomo::batch::{Operation, VerticalBatch};
use atomo::{SerdeBackend, StorageBackend, TableIndex, TableRef};
use atomo_merklized::{
    MerklizedLayout,
    MerklizedStrategy,
    SerializedStateKey,
    SerializedStateValue,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateKey,
    StateRootHash,
    StateTable,
};
use fxhash::FxHashMap;
use jmt::KeyHash;

use super::JmtTreeReader;

pub struct JmtMerklizedStrategy<L: MerklizedLayout> {
    _phantom: PhantomData<L>,
}

impl<L: MerklizedLayout> JmtMerklizedStrategy<L> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<L: MerklizedLayout> Default for JmtMerklizedStrategy<L> {
    fn default() -> Self {
        Self::new()
    }
}

impl<L: MerklizedLayout> MerklizedStrategy for JmtMerklizedStrategy<L> {
    fn get_root_hash<B: StorageBackend, S: SerdeBackend>(
        tree_table: &TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
    ) -> Result<StateRootHash> {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        tree.get_root_hash(0).map(|hash| hash.0.into())
    }

    fn get_with_proof<B: StorageBackend, S: SerdeBackend>(
        tree_table: &TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table: StateTable,
        key: SerializedStateKey,
        // TODO(snormore): Should not have to pass in the value here.
        value: Option<SerializedStateValue>,
    ) -> Result<(Option<SerializedStateValue>, Vec<u8>)> {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        // Cache the value in the reader so it can be used when `get_with_proof` is called next,
        // which calls `get_value_option`, which needs the value mapping.
        let key_hash = KeyHash(
            table
                .key(key)
                .hash::<L::SerdeBackend, L::KeyHasher>()
                .into(),
        );
        if let Some(value) = value {
            reader.cache(key_hash, L::SerdeBackend::serialize(&value));
        }
        // TODO(snormore): Fix this unwrap.
        let (value, _proof) = tree.get_with_proof(key_hash, 0).unwrap();
        // TODO(snormore): This proof should be returned
        let value = value.map(|value| L::SerdeBackend::deserialize(&value));

        // TODO(snormore): Return the real prood here, converted to something else.
        Ok((value, Vec::new()))
    }

    fn apply_changes<B: StorageBackend, S: SerdeBackend>(
        tree_table: &mut TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table_name_by_id: FxHashMap<TableIndex, String>,
        batch: VerticalBatch,
    ) -> Result<()> {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        // Iterate over the changes and build the tree value set.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        for (table_id, changes) in batch.into_raw().iter().enumerate() {
            // println!("table {:?} changes {:?}", table_id, changes);
            let table_id: TableIndex = table_id.try_into().unwrap();
            for (key, operation) in changes.iter() {
                let table_key = StateKey::new(
                    // TODO(snormore): Fix this unwrap.
                    table_name_by_id.get(&table_id).unwrap().to_string(),
                    key.to_vec().into(),
                );
                let key_hash = KeyHash(table_key.hash::<L::SerdeBackend, L::KeyHasher>().into());

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
            let key = L::SerdeBackend::serialize(&stale_node.node_key);
            // TODO(snormore): This is unecessarily/redundantly serializing. Instead, we should pass
            // in a key type that implements Serialize, because internally it serializes the key
            // again.
            tree_table.remove(key);
        }

        // New nodes are converted to insert operations.
        for (node_key, node) in tree_batch.node_batch.nodes() {
            // TODO(snormore): This is unecessarily/redundantly serializing. Instead, we should pass
            // in a key/value type that implements Serialize, because internally it serializes the
            // key/value again.
            let key = L::SerdeBackend::serialize(node_key);
            let value = L::SerdeBackend::serialize(node);
            tree_table.insert(key, value);
        }

        Ok(())
    }
}
