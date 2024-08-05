use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use trie_db::DBValue;

use super::hasher::SimpleHasherWrapper;
use super::{MptMerklizeContext, MptStateProof};
use crate::{MerklizeContext, MerklizeProvider, SimpleHasher, StateRootHash};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const ROOT_TABLE_NAME: &str = "%state_tree_root";

#[derive(Debug, Clone)]
/// A merklize provider that uses a Merkle Patricia Trie (MPT) implementation ([`mpt`]) to manage
/// the database-backed state tree.
pub struct MptMerklizeProvider<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    _phantom: PhantomData<(B, S, H)>,
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MptMerklizeProvider<B, S, H> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> Default for MptMerklizeProvider<B, S, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S, H> MerklizeProvider for MptMerklizeProvider<B, S, H>
where
    B: StorageBackend + Send + Sync,
    S: SerdeBackend + Send + Sync,
    H: SimpleHasher + Send + Sync,
{
    type Storage = B;
    type Serde = S;
    type Hasher = H;
    type Proof = MptStateProof;

    /// Build a new merklize atomo instance with the given storage backend constructor. This will
    /// create the necessary tables for the state tree nodes and keys, then build and return the
    /// atomo instance.
    fn atomo<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, S>> {
        builder
            .with_table::<<SimpleHasherWrapper<H> as hash_db::Hasher>::Out, DBValue>(
                NODES_TABLE_NAME,
            )
            .with_table::<u8, StateRootHash>(ROOT_TABLE_NAME)
            .build()
            .map_err(|e| anyhow!("Failed to build atomo instance: {:?}", e))
    }

    /// Create a new merklize context for the given table selector.
    fn context<'a>(
        ctx: &'a TableSelector<B, S>,
    ) -> Box<dyn MerklizeContext<'a, B, S, H, Self::Proof> + 'a>
    where
        H: SimpleHasher + 'a,
    {
        Box::new(MptMerklizeContext::new(ctx))
    }
}

#[cfg(test)]
mod tests {

    use atomo::{DefaultSerdeBackend, InMemoryStorage};

    use super::*;
    use crate::hashers::blake3::Blake3Hasher;
    use crate::hashers::keccak::KeccakHasher;
    use crate::hashers::sha2::Sha256Hasher;

    #[test]
    fn test_mpt_provider_blake3() {
        type S = DefaultSerdeBackend;
        type H = Blake3Hasher;
        type M = MptMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_mpt_provider_keccak256() {
        type S = DefaultSerdeBackend;
        type H = KeccakHasher;
        type M = MptMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_mpt_provider_sha256() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;
        type M = MptMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }
}
