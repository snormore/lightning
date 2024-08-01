use std::any::Any;
use std::hash::Hash;

use anyhow::Result;
use atomo::{SerdeBackend, StorageBackend};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{SimpleHasher, StateProof, StateRootHash};

pub trait DataKey: Hash + Eq + Serialize + DeserializeOwned + Any {}

pub trait DataValue: Serialize + DeserializeOwned + Any {}

/// A trait for a merklized context that can be used to interact with a merklized state tree. This
/// generally wraps and should require an atomo execution context (table selector).
pub trait MerklizedContext<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    /// Returns the root hash of the state tree.
    fn get_state_root(&self) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state. If the key exists in
    /// the state, the value and an existence proof is returned. If the key does not exist in the
    /// state, `[None]` is returned along with a non-existent proof.
    fn get_state_proof(
        &self,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<(Option<Vec<u8>>, StateProof)>;

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn apply_state_tree_changes(&mut self) -> Result<()>;
}
