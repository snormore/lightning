use atomo::TableId;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use jmt::{KeyHash, SimpleHasher};

/// Serialized value of a node in the state tree.
pub type SerializedNodeValue = Vec<u8>;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TableKey {
    pub table: TableId,
    pub key: Vec<u8>,
}

impl TableKey {
    // TODO(snormore): This is leaking `jmt::KeyHash`.
    pub fn hash<H: SimpleHasher>(&self) -> KeyHash {
        KeyHash::with::<H>(to_vec(self).unwrap())
    }
}
