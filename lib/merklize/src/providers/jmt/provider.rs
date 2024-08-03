use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use jmt::storage::{Node, NodeKey};
use jmt::KeyHash;

use super::ics23::ics23_proof_spec;
use super::JmtMerklizeContext;
use crate::{MerklizeContext, MerklizeProvider, SimpleHasher, StateKey};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const KEYS_TABLE_NAME: &str = "%state_tree_keys";

#[derive(Debug, Clone)]
/// A merklize provider that uses a Jellyfish Merkle Tree (JMT) implementation ([`jmt`]) to manage
/// the database-backed state tree.
pub struct JmtMerklizeProvider<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    _phantom: PhantomData<(B, S, H)>,
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> JmtMerklizeProvider<B, S, H> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> Default for JmtMerklizeProvider<B, S, H> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B, S, H> MerklizeProvider for JmtMerklizeProvider<B, S, H>
where
    B: StorageBackend,
    S: SerdeBackend,
    H: SimpleHasher,
{
    type Storage = B;
    type Serde = S;
    type Hasher = H;

    /// Build a new merklize atomo instance with the given storage backend constructor. This will
    /// create the necessary tables for the state tree nodes and keys, then build and return the
    /// atomo instance.
    fn atomo<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, S>> {
        builder
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .map_err(|e| anyhow!("Failed to build atomo instance: {:?}", e))
    }

    /// Create a new merklize context for the given table selector.
    fn context<'a>(ctx: &'a TableSelector<B, S>) -> Box<dyn MerklizeContext<'a, B, S, H> + 'a>
    where
        H: SimpleHasher + 'a,
    {
        Box::new(JmtMerklizeContext::new(ctx))
    }

    /// Return the ICS23 proof spec for the merklize provider, customized to the specific hasher.
    fn ics23_proof_spec() -> ics23::ProofSpec {
        ics23_proof_spec(Self::Hasher::ICS23_HASH_OP)
    }
}

#[cfg(test)]
mod tests {

    use atomo::{DefaultSerdeBackend, InMemoryStorage};

    use super::*;
    use crate::hashers::blake3::Blake3Hasher;
    use crate::hashers::keccak::KeccakHasher;
    use crate::hashers::sha2::Sha256Hasher;
    use crate::DefaultMerklizeProvider;

    #[test]
    fn test_jmt_provider_blake3() {
        type S = DefaultSerdeBackend;
        type H = Blake3Hasher;
        type M = DefaultMerklizeProvider<InMemoryStorage, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_jmt_provider_keccak256() {
        type S = DefaultSerdeBackend;
        type H = KeccakHasher;
        type M = DefaultMerklizeProvider<InMemoryStorage, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_jmt_provider_sha256() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;
        type M = DefaultMerklizeProvider<InMemoryStorage, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }
}
