use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};

use crate::{SimpleHasher, StateProof, StateRootHash};

/// A trait for a merklize provider that can be used to build a `[atomo::Atomo]` instance, and
/// provide a merklize execution context.
pub trait MerklizeProvider {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
    type Proof: StateProof;

    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn with_tables<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> AtomoBuilder<C, Self::Serde>;

    /// Returns the root hash of the state tree.
    fn get_state_root(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state. If the key exists in
    /// the state, the value and an existence proof is returned. If the key does not exist in the
    /// state, `[None]` is returned along with a non-existent proof.
    fn get_state_proof(
        ctx: &TableSelector<Self::Storage, Self::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof>;

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn apply_state_tree_changes(ctx: &TableSelector<Self::Storage, Self::Serde>) -> Result<()>;
}
