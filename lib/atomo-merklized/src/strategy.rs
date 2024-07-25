use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};

use crate::{SimpleHasher, StateRootHash};

pub trait MerklizedStrategy {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;

    /// Initialize and return an atomo instance for this strategy.
    fn build<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, Self::Serde>>;

    /// Initialize and return a new execution context using this strategy.
    fn context<'a>(
        ctx: &'a TableSelector<Self::Storage, Self::Serde>,
    ) -> Box<dyn MerklizedContext<'a, Self::Storage, Self::Serde, Self::Hasher> + 'a>
    where
        // TODO(snormore): Why is this 'a bound needed?
        Self::Hasher: SimpleHasher + 'a;
}

pub trait MerklizedContext<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    /// Returns the root hash of the state tree.
    fn get_state_root(&self) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state. If the key exists in
    /// the state, the value and an existence proof is returned. If the key does not exist in the
    /// state, `[None]` is returned along with a non-existent proof.
    fn get_state_proof(
        &self,
        table: &str,
        // TODO(snormore): Can we use generic key/value types here?
        serialized_key: Vec<u8>,
    ) -> Result<(Option<Vec<u8>>, ics23::CommitmentProof)>;

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn apply_state_tree_changes(&mut self) -> Result<()>;
}
