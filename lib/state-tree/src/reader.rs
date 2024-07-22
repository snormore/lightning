use std::marker::PhantomData;

use anyhow::Result;
use atomo::{Atomo, QueryPerm, SerdeBackend, StorageBackend};
use jmt::{RootHash, SimpleHasher};

use crate::{SerializedNodeKey, SerializedNodeValue, StateTreeTableSelector};

// TODO(snormore): This is leaking `jmt::SimpleHasher`.`
pub struct StateTreeReader<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher> {
    inner: Atomo<QueryPerm, B, S>,
    tree_table_name: String,
    _phantom: PhantomData<(B, S, KH, VH)>,
}

impl<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
    StateTreeReader<B, S, KH, VH>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    pub fn new(inner: Atomo<QueryPerm, B, S>, tree_table_name: String) -> Self {
        Self {
            inner,
            tree_table_name,
            _phantom: PhantomData,
        }
    }

    /// Run a query on the database.
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut StateTreeTableSelector<B, S, KH, VH>) -> R,
    {
        self.inner.run(|ctx| {
            let tree_table = ctx
                .get_table::<SerializedNodeKey, SerializedNodeValue>(self.tree_table_name.clone());
            let mut ctx = StateTreeTableSelector::new(ctx, &tree_table);
            query(&mut ctx)
        })
    }

    /// Return the state root hash of the state tree.
    // TODO(snormore): This is leaking `jmt::RootHash`.`
    pub fn get_state_root(&self) -> Result<RootHash> {
        self.run(|ctx| ctx.get_state_root())
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
