use std::marker::PhantomData;

use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use jmt::storage::{Node, NodeKey};
use jmt::KeyHash;

use super::proof::JmtStateProof;
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
    type Proof = JmtStateProof;

    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn with_tables<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> AtomoBuilder<C, S> {
        builder
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
    }

    /// Create a new merklize context for the given table selector.
    fn context<'a>(
        ctx: &'a TableSelector<B, S>,
    ) -> Box<dyn MerklizeContext<'a, B, S, H, Self::Proof> + 'a>
    where
        H: SimpleHasher + 'a,
    {
        Box::new(JmtMerklizeContext::new(ctx))
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
    fn test_jmt_provider_blake3() {
        type H = Blake3Hasher;
        type M = JmtMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder =
            AtomoBuilder::new(InMemoryStorage::default()).with_table::<String, String>("data");
        let db = M::with_tables(builder).build().unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_jmt_provider_keccak256() {
        type H = KeccakHasher;
        type M = JmtMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder =
            AtomoBuilder::new(InMemoryStorage::default()).with_table::<String, String>("data");
        let db = M::with_tables(builder).build().unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_jmt_provider_sha256() {
        type H = Sha256Hasher;
        type M = JmtMerklizeProvider<InMemoryStorage, DefaultSerdeBackend, H>;

        let builder =
            AtomoBuilder::new(InMemoryStorage::default()).with_table::<String, String>("data");
        let db = M::with_tables(builder).build().unwrap();
        let _query = db.query();
    }
}
