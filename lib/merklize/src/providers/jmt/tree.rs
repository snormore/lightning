use std::marker::PhantomData;

use atomo::{SerdeBackend, StorageBackendConstructor};
use jmt::Version;

use super::proof::JmtStateProof;
use super::{JmtStateTreeBuilder, JmtStateTreeReader, JmtStateTreeWriter};
use crate::{SimpleHasher, StateTree};

pub(crate) const NODES_TABLE_NAME: &str = "%state_tree_nodes";
pub(crate) const KEYS_TABLE_NAME: &str = "%state_tree_keys";

// The version of the JMT state tree.
// This needs to be greater than 0 because of the way we use the `jmt` crate without versioning. In
// `update_state_tree`, we insert the root node with version minus 1 to satisfy `jmt` crate
// expectations of retrieving the root of the previous version, which will panic if the version is
// 0. The `jmt` crate also has special handling of version 0, which we don't want to be in effect.
pub(crate) const TREE_VERSION: Version = 1;

#[derive(Debug, Clone)]
/// A merklize provider that uses a Jellyfish Merkle Tree (JMT) implementation ([`jmt`]) to manage
/// the database-backed state tree.
pub struct JmtStateTree<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> {
    _storage: PhantomData<B>,
    _serde: PhantomData<S>,
    _hasher: PhantomData<H>,
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> JmtStateTree<B, S, H> {
    pub fn new() -> Self {
        Self {
            _storage: PhantomData,
            _serde: PhantomData,
            _hasher: PhantomData,
        }
    }
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> Default
    for JmtStateTree<B, S, H>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> StateTree
    for JmtStateTree<B, S, H>
{
    type StorageBuilder = B;
    type Serde = S;
    type Hasher = H;
    type Proof = JmtStateProof;

    type Builder = JmtStateTreeBuilder<Self>;
    type Reader = JmtStateTreeReader<Self>;
    type Writer = JmtStateTreeWriter<Self>;

    fn builder(&self) -> Self::Builder {
        JmtStateTreeBuilder::new()
    }

    fn reader(&self) -> Self::Reader {
        JmtStateTreeReader::new()
    }

    fn writer(&self) -> Self::Writer {
        JmtStateTreeWriter::new()
    }
}
