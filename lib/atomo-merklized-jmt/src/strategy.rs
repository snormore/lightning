use std::marker::PhantomData;

use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use atomo_merklized::{MerklizedContext, MerklizedStrategy, SimpleHasher, StateKey};
use jmt::KeyHash;

use crate::JmtMerklizedContext;

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
            // TODO(snormore): Fix these hard coded table names.
            .with_table::<Vec<u8>, Vec<u8>>("%state_tree_nodes")
            .with_table::<KeyHash, StateKey>("%state_tree_keys")
            .with_table::<KeyHash, Vec<u8>>("%state_tree_values")
            // TODO(snormore): This `enable_iter` is unecessary and is only here for testing right
            // now. It should be removed.
            .enable_iter("%state_tree_nodes")
            .enable_iter("%state_tree_keys")
            .build()
            .unwrap())
    }

    fn context<'a>(ctx: &'a TableSelector<B, S>) -> Box<dyn MerklizedContext<'a, B, S, H> + 'a>
    where
        // TODO(snormore): Why is this needed?
        H: 'a,
    {
        Box::new(JmtMerklizedContext::new(ctx))
    }
}
