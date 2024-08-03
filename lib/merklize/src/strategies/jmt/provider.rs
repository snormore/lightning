use std::marker::PhantomData;

use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};
use jmt::storage::{Node, NodeKey};
use jmt::KeyHash;

use super::ics23::ics23_proof_spec;
use super::JmtMerklizedContext;
use crate::{MerklizeProvider, MerklizedContext, SimpleHasher, StateKey};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const KEYS_TABLE_NAME: &str = "%state_tree_keys";

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

impl<B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MerklizeProvider
    for JmtMerklizeProvider<B, S, H>
{
    type Storage = B;
    type Serde = S;
    type Hasher = H;

    /// Build a new merklized atomo instance with the given storage backend constructor. This will
    /// create the necessary tables for the state tree nodes and keys, then build and return the
    /// atomo instance.
    fn atomo<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, S>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, S>> {
        Ok(builder
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .unwrap())
    }

    /// Create a new merklized context for the given table selector.
    fn context<'a>(ctx: &'a TableSelector<B, S>) -> Box<dyn MerklizedContext<'a, B, S, H> + 'a>
    where
        H: SimpleHasher + 'a,
    {
        Box::new(JmtMerklizedContext::new(ctx))
    }

    /// Return the ICS23 proof spec for the merklize provider, customized to the specific hasher.
    fn ics23_proof_spec() -> ics23::ProofSpec {
        ics23_proof_spec(Self::Hasher::ICS23_HASH_OP)
    }
}

#[cfg(test)]
mod tests {

    use atomo::{DefaultSerdeBackend, InMemoryStorage};
    use atomo_rocks::{Options, RocksBackend, RocksBackendBuilder};
    use tempfile::tempdir;

    use super::*;
    use crate::hashers::sha2::Sha256Hasher;
    use crate::DefaultMerklizeProvider;

    #[test]
    fn test_atomo_memdb_sha256() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;
        type M = DefaultMerklizeProvider<InMemoryStorage, H>;

        let builder = InMemoryStorage::default();
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }

    #[test]
    fn test_atomo_rocksdb_sha256() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;
        type M = DefaultMerklizeProvider<RocksBackend, H>;

        let temp_dir = tempdir().unwrap();
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);
        let builder = RocksBackendBuilder::new(temp_dir.path()).with_options(options);
        let db = M::atomo(AtomoBuilder::<_, S>::new(builder).with_table::<String, String>("data"))
            .unwrap();
        let _query = db.query();
    }
}
