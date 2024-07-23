use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::proof::SparseMerkleProof;
use jmt::{RootHash, SimpleHasher};

use crate::{SerializedNodeKey, SerializedNodeValue};

/// A strategy for a merklized atomo describing the configuration and architecture of the
/// database-backed merkle state tree.
pub trait MerklizedStrategy<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
{
    // fn build(tree_table: TableRef<SerializedNodeKey, SerializedNodeValue, B, S>) -> &Self;

    /// Returns the `[atomo::TableRef]` for the state tree data.
    fn tree_table(&self) -> &TableRef<SerializedNodeKey, SerializedNodeValue, B, S>;

    /// Returns the root hash of the state tree.
    fn get_root_hash(&self) -> Result<RootHash>;

    /// Generates and returns a merkle proof for the given key in the state. If the key exists in
    /// the state, the value and an existence proof is returned. If the key does not exist in the
    /// state, `[None]` is returned along with a non-existent proof.
    fn get_with_proof(
        &self,
        table: String,
        key: Vec<u8>,
        value: Option<Vec<u8>>,
    ) -> Result<(Option<Vec<u8>>, SparseMerkleProof<VH>)>;

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn apply_changes(&mut self, batch: VerticalBatch) -> Result<()>;
}
