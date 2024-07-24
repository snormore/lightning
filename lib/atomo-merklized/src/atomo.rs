use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use anyhow::Result;
use atomo::{
    Atomo,
    DefaultSerdeBackend,
    QueryPerm,
    ResolvedTableReference,
    SerdeBackend,
    StorageBackend,
    TableId,
    UpdatePerm,
};
use fxhash::FxHashMap;
use jmt::SimpleHasher;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::jmt::JmtMerklizedStrategy;
use crate::types::{SerializedTreeNodeKey, SerializedTreeNodeValue};
use crate::{KeccakHasher, MerklizedStrategy, MerklizedTableSelector, RootHash};

// TODO(snormore): This is leaking `jmt::SimpleHasher`.
pub struct MerklizedAtomo<
    P,
    B: StorageBackend,
    S: SerdeBackend = DefaultSerdeBackend,
    // TODO(snormore): Move the hashers into a layout or in the strategy. We shouldn't be
    // defaulting here.
    KH: SimpleHasher = blake3::Hasher,
    VH: SimpleHasher = KeccakHasher,
    // X: MerklizedAtomoStrategy<B, S, KH, VH>,
> {
    inner: Atomo<P, B, S>,
    tree_table_name: String,
    table_name_by_id: FxHashMap<TableId, String>,
    table_id_by_name: FxHashMap<String, TableId>,
    _phantom: PhantomData<(KH, VH)>,
}

/// Implement the `Clone` trait for `MerklizedAtomo<QueryPerm>`.
impl<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedAtomoStrategy<B, S, KH, VH>,
> Clone for MerklizedAtomo<QueryPerm, B, S, KH, VH>
where
    B: StorageBackend,
    S: SerdeBackend,
{
    fn clone(&self) -> Self {
        Self::new(
            self.inner.clone(),
            self.tree_table_name.clone(),
            self.table_id_by_name.clone(),
        )
    }
}

impl<
    P,
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedAtomoStrategy<B, S, KH, VH>,
> MerklizedAtomo<P, B, S, KH, VH>
{
    pub fn new(
        inner: Atomo<P, B, S>,
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
            table_id_by_name,
            _phantom: PhantomData,
        }
    }

    /// Build and return a query reader for the data.
    pub fn query(&self) -> MerklizedAtomo<QueryPerm, B, S, KH, VH> {
        MerklizedAtomo::new(
            self.inner.query(),
            self.tree_table_name.clone(),
            self.table_id_by_name.clone(),
        )
    }

    /// Resolve a table with the given name and key-value types.
    pub fn resolve<K, V>(&self, name: impl AsRef<str>) -> ResolvedTableReference<K, V>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        self.inner.resolve::<K, V>(name)
    }
}

impl<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedAtomoStrategy<B, S, KH, VH>,
> MerklizedAtomo<UpdatePerm, B, S, KH, VH>
{
    /// Run an update on the data.
    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(
            &mut MerklizedTableSelector<B, S, KH, VH, JmtMerklizedStrategy<B, S, KH, VH>>,
        ) -> R,
    {
        let tree_table_name = self.tree_table_name.clone();
        self.inner.run(|ctx| {
            let tree_table =
                ctx.get_table::<SerializedTreeNodeKey, SerializedTreeNodeValue>(tree_table_name);
            // TODO(snormore): Strategy builder should be passed in here instead of the
            // implementation being hard coded.
            // let strategy = X::build(tree_table);
            let mut strategy = JmtMerklizedStrategy::new(tree_table, self.table_name_by_id.clone());
            let mut ctx = MerklizedTableSelector::new(ctx, &strategy);
            let res = mutation(&mut ctx);

            // TODO(snormore): Fix this unwrap.
            strategy.apply_changes(ctx.current_changes()).unwrap();

            res
        })
    }

    /// Return the internal storage backend.
    pub fn get_storage_backend_unsafe(&mut self) -> &B {
        self.inner.get_storage_backend_unsafe()
    }
}

impl<
    B: StorageBackend,
    S: SerdeBackend,
    KH: SimpleHasher,
    VH: SimpleHasher,
    // X: MerklizedStrategy<B, S, KH, VH>,
> MerklizedAtomo<QueryPerm, B, S, KH, VH>
{
    /// Run a query on the database.
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(
            &mut MerklizedTableSelector<B, S, KH, VH, JmtMerklizedStrategy<B, S, KH, VH>>,
        ) -> R,
    {
        self.inner.run(|ctx| {
            let tree_table = ctx
                .get_table::<SerializedTreeNodeKey, SerializedTreeNodeValue>(self.tree_table_name.clone());
            // let strategy = X::build(tree_table);
            let strategy = JmtMerklizedStrategy::new(tree_table, self.table_name_by_id.clone());
            let mut ctx = MerklizedTableSelector::new(ctx, &strategy);
            query(&mut ctx)
        })
    }

    /// Return the state root hash of the state tree.
    pub fn get_state_root(&self) -> Result<RootHash> {
        self.run(|ctx| ctx.get_state_root())
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
//             MerklizedAtomoWriter::<_, KeyHasher, ValueHasher>::new(storage.clone(),
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
