use std::marker::PhantomData;

use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use atomo_merklized::{MerklizedContext, MerklizedStrategy, SimpleHasher, StateKey};
use jmt::KeyHash;

use crate::JmtMerklizedContext;

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const KEYS_TABLE_NAME: &str = "%state_tree_keys";

pub struct JmtMerklizedStrategy<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    _phantom: PhantomData<(B, S, H)>,
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> JmtMerklizedStrategy<B, S, H> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> Default
    for JmtMerklizedStrategy<B, S, H>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MerklizedStrategy
    for JmtMerklizedStrategy<B, S, H>
{
    type Storage = B;
    type Serde = S;
    type Hasher = H;

    fn build<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, S>> {
        Ok(builder
            .with_table::<Vec<u8>, Vec<u8>>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .unwrap())
    }

    fn context<'a>(ctx: &'a TableSelector<B, S>) -> Box<dyn MerklizedContext<'a, B, S, H> + 'a>
    where
        H: SimpleHasher + 'a,
    {
        Box::new(JmtMerklizedContext::new(ctx))
    }
}
