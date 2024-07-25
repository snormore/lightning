// use std::any::Any;
// use std::borrow::Borrow;
// use std::hash::Hash;
// use std::marker::PhantomData;

// use anyhow::Result;
// use atomo::{SerdeBackend, StorageBackend, TableRef};
// use serde::de::DeserializeOwned;
// use serde::Serialize;

// use crate::{MerklizedContext, MerklizedStrategy, StateRootHash, StateTable};

// /// A selector for tables in a merklized atomo, that can be used to query and update the tables.
// It /// wraps an atomo table selector and a reference to the state tree table.
// pub struct MerklizedTableSelector<
//     'a,
//     B: StorageBackend,
//     S: SerdeBackend,
//     H: SimpleHasher,
//     X: MerklizedStrategy<B, S, H>,
// > { inner: &'a atomo::TableSelector<B, S>, ctx: Box<dyn MerklizedContext<'a, B, S, H> + 'a>,
// > _phantom: PhantomData<X>,
// }

// impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher, X: MerklizedStrategy<B, S, H>>
//     MerklizedTableSelector<'a, B, S, H, X>
// where
//     H: 'a,
// {
//     /// Create a new table selector.
//     pub fn new(inner: &'a atomo::TableSelector<B, S>) -> Self {
//         let ctx = X::context(inner);
//         Self {
//             inner,
//             ctx,
//             _phantom: PhantomData,
//         }
//     }

//     /// Returns the inner atomo table selector.
//     #[inline]
//     pub fn inner(&self) -> &'a atomo::TableSelector<B, S> {
//         self.inner
//     }

//     /// Apply the changes in the batch to the state tree.
//     pub fn apply_state_tree_changes(&mut self) -> Result<()> {
//         self.ctx.apply_changes(self.inner.batch())
//     }

//     /// Return the table reference for the table with the provided name and K, V type.
//     #[inline]
//     pub fn get_table<K, V>(&self, table: impl AsRef<str>) -> TableRef<'a, K, V, B, S>
//     where
//         K: Hash + Eq + Serialize + DeserializeOwned + Any,
//         V: Serialize + DeserializeOwned + Any,
//     {
//         self.inner.get_table(table.as_ref())
//     }

//     /// Return the value associated with the provided key, along with a merkle proof of existence
// in     /// the state tree. If the key doesn't exist in the table, [`None`] is returned.
//     pub fn get_state_proof<K, V>(
//         &self,
//         table: impl AsRef<str>,
//         key: impl Borrow<K>,
//     ) -> (Option<V>, ics23::CommitmentProof)
//     where
//         K: Hash + Eq + Serialize + DeserializeOwned + Any,
//         V: Serialize + DeserializeOwned + Any,
//     {
//         let table = StateTable::new(table);
//         let serialized_key = S::serialize(&key.borrow());
//         let (value, proof) = self.ctx.get_proof(table, serialized_key).unwrap();
//         // TODO(snormore): Fix this unwrap.
//         let value = value.map(|v| S::deserialize::<V>(v.as_slice()));
//         (value, proof)
//     }

//     /// Return the state root hash of the state tree.
//     #[inline]
//     pub fn get_state_root(&self) -> Result<StateRootHash> {
//         self.ctx.get_root()
//     }
// }
