use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableRef};
use jmt::proof::SparseMerkleProof;
use jmt::{RootHash, SimpleHasher};

use crate::{SerializedNodeKey, SerializedNodeValue};

pub trait StateTreeStrategy<B: StorageBackend, S: SerdeBackend, KH: SimpleHasher, VH: SimpleHasher>
{
    // fn build(tree_table: TableRef<SerializedNodeKey, SerializedNodeValue, B, S>) -> &Self;

    fn tree_table(&self) -> &TableRef<SerializedNodeKey, SerializedNodeValue, B, S>;

    fn get_root_hash(&self) -> Result<RootHash>;
    fn get_with_proof(
        &self,
        table: String,
        key: Vec<u8>,
        value: Option<Vec<u8>>,
    ) -> Result<(Option<Vec<u8>>, SparseMerkleProof<VH>)>;
    fn apply_changes(&mut self, batch: VerticalBatch) -> Result<()>;
}
