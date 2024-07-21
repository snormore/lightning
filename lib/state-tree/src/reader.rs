use std::marker::PhantomData;

use anyhow::Result;
use atomo::{SerdeBackend, StorageBackend, TableSelector};
use jmt::proof::SparseMerkleProof;
use jmt::{RootHash, SimpleHasher};

use crate::jmt::JmtTreeReader;
use crate::types::{SerializedNodeValue, TableKey};

// TODO(snormore): This is leaking `jmt::SimpleHasher`.`
pub struct StateTreeReader<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher> {
    tree_table_name: String,
    _phantom: PhantomData<(B, S, KH, VH)>,
}

impl<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeReader<B, S, KH, VH>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    pub fn new(tree_table_name: String) -> Self {
        Self {
            tree_table_name,
            _phantom: PhantomData,
        }
    }

    // TODO(snormore): This is leaking `jmt::RootHash`.`
    pub fn get_root_hash(&self, ctx: &TableSelector<B, S>) -> Result<RootHash> {
        let reader = JmtTreeReader::new(ctx, self.tree_table_name.clone());
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        tree.get_root_hash(0)
    }

    /// Get the value of a key in the state tree, along with a merkle proof that can be used to
    /// verify existence.
    // TODO(snormore): This is leaking `jmt::SparseMerkleProof`.
    pub fn get_with_proof(
        &self,
        ctx: &TableSelector<B, S>,
        key: TableKey,
    ) -> Result<(Option<SerializedNodeValue>, SparseMerkleProof<VH>)> {
        let reader = JmtTreeReader::new(ctx, self.tree_table_name.clone());
        let tree = jmt::JellyfishMerkleTree::<_, VH>::new(&reader);

        // Cache the key in the reader so it can be used when `get_with_proof` is called next, which
        // calls `get_value_option`.`
        let key_hash = key.hash::<KH>();
        reader.cache_key(key_hash, key);

        tree.get_with_proof(key_hash, 0)
    }
}

// #[cfg(test)]
// mod tests {
//     use std::collections::HashMap;
//     use std::vec;

//     use atomo::batch::{Operation, VerticalBatch};
//     use atomo::{InMemoryStorage, StorageBackendConstructor};

//     use super::*;
//     use crate::keccak::KeccakHasher;
//     use crate::StateTreeWriter;

//     #[test]
//     fn test_get_root_hash() {
//         type KeyHasher = blake3::Hasher;
//         type ValueHasher = KeccakHasher;

//         let mut storage = InMemoryStorage::default();
//         let data_table_id = storage.open_table("data".to_string());
//         let tree_table_id = storage.open_table("tree".to_string());
//         let storage = Arc::new(storage);

//         let writer =
//             StateTreeWriter::<_, KeyHasher, ValueHasher>::new(storage.clone(),
// "tree".to_string());         let reader =
//             StateTreeReader::<_, KeyHasher, ValueHasher>::new(storage.clone(),
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

//         writer.commit(batch);

//         let root_hash = reader.get_root_hash().unwrap();
//         assert_ne!(root_hash.as_ref(), [0; 32]);
//         assert_eq!(
//             hex::encode(root_hash.as_ref()),
//             "6111f6c29d8c8b704636573e6822c68d4271263a5fcf92ad17f88557a7d132ab"
//         );
//     }
// }
