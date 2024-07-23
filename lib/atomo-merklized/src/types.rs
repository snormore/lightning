use atomo::SerdeBackend;
use jmt::{KeyHash, SimpleHasher};
use serde::{Deserialize, Serialize};

/// Serialized key of a node in the state tree.
pub type SerializedNodeKey = Vec<u8>;

/// Serialized value of a node in the state tree.
pub type SerializedNodeValue = Vec<u8>;

/// Encapsulation of a value (leaf node) key in the state tree, including the state table name and
/// entry key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableKey {
    // TODO(snormore): Make this an enum?
    pub table: String,
    pub key: Vec<u8>,
}

impl TableKey {
    // TODO(snormore): This is leaking `jmt::KeyHash`.
    pub fn hash<S: SerdeBackend, H: SimpleHasher>(&self) -> KeyHash {
        KeyHash::with::<H>(S::serialize(&self))
    }
}
