use std::marker::PhantomData;

use anyhow::Result;
use atomo::{Atomo, QueryPerm, SerdeBackend, StorageBackend, TableId};
use fxhash::FxHashMap;
use jmt::{RootHash, SimpleHasher};

use crate::jmt::JmtMerklizedStrategy;
use crate::{MerklizedTableSelector, SerializedNodeKey, SerializedNodeValue};

/// An atomo reader that can be used to also query the merklized state tree. It is a wrapper of
/// `[atomo::Atomo<QueryPerm>]`.
pub struct MerklizedAtomoReader<
    B: StorageBackend,
    S: SerdeBackend,
    // TODO(snormore): This is leaking `jmt::SimpleHasher`.`
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedStrategy<B, S, KH, VH>,
> {
    inner: Atomo<QueryPerm, B, S>,
    tree_table_name: String,
    table_name_by_id: FxHashMap<TableId, String>,
    _phantom: PhantomData<(B, S, KH, VH)>,
}

impl<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedStrategy<B, S, KH, VH>,
> MerklizedAtomoReader<B, S, KH, VH>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
{
    /// Create and return a new reader.
    pub fn new(
        inner: Atomo<QueryPerm, B, S>,
        tree_table_name: String,
        table_name_by_id: FxHashMap<TableId, String>,
    ) -> Self {
        Self {
            inner,
            tree_table_name,
            table_name_by_id,
            _phantom: PhantomData,
        }
    }

    /// Run a query on the database.
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(
            &mut MerklizedTableSelector<B, S, KH, VH, JmtMerklizedStrategy<B, S, KH, VH>>,
        ) -> R,
    {
        self.inner.run(|ctx| {
            let tree_table = ctx
                .get_table::<SerializedNodeKey, SerializedNodeValue>(self.tree_table_name.clone());
            // let strategy = X::build(tree_table);
            let strategy = JmtMerklizedStrategy::new(tree_table, self.table_name_by_id.clone());
            let mut ctx = MerklizedTableSelector::new(ctx, &strategy);
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
//     use crate::MerklizedAtomoWriter;

//     #[test]
//     fn test_get_root_hash() {
//         type KeyHasher = blake3::Hasher;
//         type ValueHasher = KeccakHasher;

//         let mut storage = InMemoryStorage::default();
//         let data_table_id = storage.open_table("data".to_string());
//         let tree_table_id = storage.open_table("tree".to_string());
//         let storage = Arc::new(storage);

//         let writer =
//             MerklizedAtomoWriter::<_, KeyHasher, ValueHasher>::new(storage.clone(),
// "tree".to_string());         let reader =
//             MerklizedAtomoReader::<_, KeyHasher, ValueHasher>::new(storage.clone(),
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
