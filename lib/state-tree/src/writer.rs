use std::marker::PhantomData;

use atomo::{Atomo, SerdeBackend, StorageBackend, TableId, UpdatePerm};
use fxhash::FxHashMap;
use jmt::SimpleHasher;

use crate::jmt::JmtStateTreeStrategy;
use crate::types::{SerializedNodeKey, SerializedNodeValue};
use crate::{StateTreeReader, StateTreeStrategy, StateTreeTableSelector};

// TODO(snormore): This is leaking `jmt::SimpleHasher`.
pub struct StateTreeWriter<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: StateTreeStrategy<B, S, KH, VH>,
> {
    inner: Atomo<UpdatePerm, B, S>,
    tree_table_name: String,
    table_name_by_id: FxHashMap<TableId, String>,
    _phantom: PhantomData<(KH, VH)>,
}

impl<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: StateTreeStrategy<B, S, KH, VH>,
> StateTreeWriter<B, S, KH, VH>
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
        F: FnOnce(
            &mut StateTreeTableSelector<B, S, KH, VH, JmtStateTreeStrategy<B, S, KH, VH>>,
        ) -> R,
    {
        let tree_table_name = self.tree_table_name.clone();
        self.inner.run(|ctx| {
            let tree_table =
                ctx.get_table::<SerializedNodeKey, SerializedNodeValue>(tree_table_name);
            // TODO(snormore): Strategy builder should be passed in here instead of the
            // implementation being hard coded.
            // let strategy = X::build(tree_table);
            let mut strategy = JmtStateTreeStrategy::new(tree_table, self.table_name_by_id.clone());
            let mut ctx = StateTreeTableSelector::new(ctx, &strategy);
            let res = mutation(&mut ctx);

            // TODO(snormore): Fix this unwrap.
            strategy.apply_changes(ctx.current_changes()).unwrap();

            res
        })
    }

    /// Build and return a query reader for the data.
    pub fn query(&self) -> StateTreeReader<B, S, KH, VH> {
        StateTreeReader::new(
            self.inner.query(),
            self.tree_table_name.clone(),
            self.table_name_by_id.clone(),
        )
    }

    /// Return the internal storage backend.
    pub fn get_storage_backend_unsafe(&mut self) -> &B {
        self.inner.get_storage_backend_unsafe()
    }
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
