use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use atomo::batch::{Operation, VerticalBatch};
use atomo::{SerdeBackend, StorageBackend, TableIndex, TableRef};
use atomo_merklized::{
    MerklizedLayout,
    MerklizedStrategy,
    SerializedTreeNodeKey,
    SerializedTreeNodeValue,
    StateKey,
    StateRootHash,
    StateTable,
};
use fxhash::FxHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

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
    fn get_root<B: StorageBackend, S: SerdeBackend>(
        tree_table: &TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
    ) -> Result<StateRootHash> {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        tree.get_root_hash(0).map(|hash| hash.0.into())
    }

    // TODO(snormore): Pass <K, V> in here and serialize/deserialize the key and value instead of
    // the special Serialized* types.
    // TODO(snormore): Return a proof type instead of a `Vec<u8>`, or something standard like an
    // ics23 proof.
    fn get_proof<K, V, B: StorageBackend, S: SerdeBackend>(
        tree_table: &TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table: StateTable,
        key: impl Borrow<K>,
        value: Option<V>,
    ) -> Result<(Option<V>, Vec<u8>)>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        // Cache the value with the reader so it can be retrieved when `get_with_proof` is called
        // after this.
        let key_hash = jmt::KeyHash(
            table
                .key(S::serialize(key.borrow()).into())
                .hash::<L::SerdeBackend, L::KeyHasher>()
                .into(),
        );
        if let Some(value) = value {
            reader.cache(key_hash, L::SerdeBackend::serialize(&value));
        }

        // Get the value and proof from the tree.
        let (value, proof) = tree.get_with_proof(key_hash, 0)?;
        let value = value.map(|value| L::SerdeBackend::deserialize(&value));
        // TODO(snormore): Build and return the proof here in our own type instead of opaquely
        // serializing to bytes.
        let proof = L::SerdeBackend::serialize(&proof);

        Ok((value, proof))
    }

    fn apply_changes<B: StorageBackend, S: SerdeBackend>(
        tree_table: &mut TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table_name_by_id: FxHashMap<TableIndex, String>,
        batch: VerticalBatch,
    ) -> Result<()> {
        let reader = JmtTreeReader::new(tree_table);
        let tree = jmt::JellyfishMerkleTree::<_, L::ValueHasher>::new(&reader);

        // Build a jmt value set (batch) from the atomo batch.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        for (table_id, changes) in batch.into_raw().iter().enumerate() {
            let table_id: TableIndex = table_id.try_into()?;
            let table_name = table_name_by_id
                .get(&table_id)
                .ok_or(anyhow!("Table with index {} not found", table_id))?
                .as_str();
            for (key, operation) in changes.iter() {
                let table_key = StateKey::new(table_name.to_string(), key.to_vec().into());
                let key_hash =
                    jmt::KeyHash(table_key.hash::<L::SerdeBackend, L::KeyHasher>().into());

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

        // Apply the jmt value set (batch) to the tree.
        let (_new_root_hash, tree_batch) = tree.put_value_set(value_set.clone(), 0).unwrap();

        // Remove stale nodes.
        for stale_node in tree_batch.stale_node_index_batch {
            let key = L::SerdeBackend::serialize(&stale_node.node_key);
            tree_table.remove(key);
        }

        // Insert new nodes.
        for (node_key, node) in tree_batch.node_batch.nodes() {
            let key = L::SerdeBackend::serialize(node_key);
            let value = L::SerdeBackend::serialize(node);
            tree_table.insert(key, value);
        }

        Ok(())
    }
}
