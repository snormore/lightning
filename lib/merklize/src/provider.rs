use anyhow::Result;
use atomo::{
    Atomo,
    AtomoBuilder,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableSelector,
    UpdatePerm,
};

use crate::{SimpleHasher, StateProof, StateRootHash};

/// A trait for a merklize provider used to maintain and interact with the state tree.
///
/// ## Example
///
/// ```rust
#[doc = include_str!("../examples/jmt-sha256.rs")]
/// ```
pub trait MerklizeProvider {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
    type Proof: StateProof;

    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn with_tables<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> AtomoBuilder<C, Self::Serde>;

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn update_state_tree(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<()>;

    /// Returns the root hash of the state tree.
    fn get_state_root(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state.
    fn get_state_proof(
        ctx: &TableSelector<Self::Storage, Self::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof>;

    /// Build a temporary state tree from the full state, and return the root hash.
    /// This can be used to perform a full integrity check of the stored state, by rebuilding the
    /// state tree from the full state and comparing with this method, and comparing the returned
    /// root hash with the expected root hash.
    fn build_state_root(
        db: &mut Atomo<UpdatePerm, Self::Storage, Self::Serde>,
    ) -> Result<StateRootHash>;
}
