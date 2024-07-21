use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use jmt::{KeyHash, SimpleHasher};

/// Serialized key of a node in the state tree.
pub type SerializedNodeKey = Vec<u8>;

/// Serialized value of a node in the state tree.
pub type SerializedNodeValue = Vec<u8>;

/// Encapsulation of a value (leaf node) key in the state tree, including the state table name and
/// entry key.
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct TableKey {
    // TODO(snormore): Make this an enum?
    pub table: String,
    pub key: Vec<u8>,
}

impl TableKey {
    // TODO(snormore): This is leaking `jmt::KeyHash`.
    pub fn hash<H: SimpleHasher>(&self) -> KeyHash {
        KeyHash::with::<H>(to_vec(self).unwrap())
    }
}
