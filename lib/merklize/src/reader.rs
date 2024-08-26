use anyhow::Result;
use atomo::{Atomo, QueryPerm, SerdeBackend, StorageBackend, TableSelector};

use crate::{SimpleHasher, StateProof, StateRootHash};

pub trait StateTreeReader<B: StorageBackend, S: SerdeBackend, H: SimpleHasher>:
    Sized + Send + Sync + Clone
{
    type Proof: StateProof;

    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to get the root hash of the state
    ///   tree.
    fn get_state_root(&self, ctx: &TableSelector<B, S>) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state.
    ///
    /// This method uses an atomo execution context, so it is safe to use concurrently.
    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to generate the proof.
    /// - `table`: The name of the table to generate the proof for.
    /// - `serialized_key`: The serialized key to generate the proof for.
    fn get_state_proof(
        &self,
        ctx: &TableSelector<B, S>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof>;

    /// Verifies that the state in the given atomo database instance, when used to build a
    /// new, temporary state tree from scratch, matches the stored state tree root hash.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to verify.
    fn verify_state_tree_unsafe(&self, db: &mut Atomo<QueryPerm, B, S>) -> Result<()>;

    /// Returns whether the state tree is empty.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to check.
    // TODO(snormore): Can we do this without mut self?
    fn is_empty_state_tree_unsafe(&self, db: &mut Atomo<QueryPerm, B, S>) -> Result<bool>;
}
