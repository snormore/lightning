use atomo::SerdeBackend;
// TODO(snormore): Move `hex_array` to a separate, common crate, like utils, and use it here
// instead of dependending on `fleek-crypto` (and remove the dependency from `Cargo.toml`).
use fleek_crypto::hex_array;
use jmt::{KeyHash, SimpleHasher};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Root hash of the state tree.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct RootHash(
    #[serde(
        deserialize_with = "hex_array::deserialize",
        serialize_with = "hex_array::serialize"
    )]
    [u8; 32],
);

impl From<[u8; 32]> for RootHash {
    fn from(hash: [u8; 32]) -> Self {
        Self(hash)
    }
}

impl From<RootHash> for [u8; 32] {
    fn from(hash: RootHash) -> Self {
        hash.0
    }
}

impl std::fmt::Display for RootHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).unwrap().trim_matches('"')
        )
    }
}

impl PartialEq<&str> for RootHash {
    fn eq(&self, other: &&str) -> bool {
        &self.to_string() == other
    }
}

/// Serialized key of a node in the state tree.
pub type SerializedTreeNodeKey = Vec<u8>;

/// Serialized value of a node in the state tree.
pub type SerializedTreeNodeValue = Vec<u8>;

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
