use atomo::SerdeBackend;
// TODO(snormore): Move `hex_array` to a separate, common crate, like utils, and use it here
// instead of dependending on `fleek-crypto` (and remove the dependency from `Cargo.toml`).
use fleek_crypto::hex_array;
use jmt::proof::SparseMerkleProof;
use jmt::SimpleHasher;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Root hash of the state tree.
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
pub struct SimpleHash(
    #[serde(
        deserialize_with = "hex_array::deserialize",
        serialize_with = "hex_array::serialize"
    )]
    [u8; 32],
);

impl SimpleHash {
    pub fn build<H: SimpleHasher>(key: impl AsRef<[u8]>) -> Self {
        Self(H::hash(key.as_ref()))
    }
}
impl From<[u8; 32]> for SimpleHash {
    fn from(hash: [u8; 32]) -> Self {
        Self(hash)
    }
}

impl From<SimpleHash> for [u8; 32] {
    fn from(hash: SimpleHash) -> Self {
        hash.0
    }
}

impl AsRef<[u8]> for SimpleHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<&str> for SimpleHash {
    fn eq(&self, other: &&str) -> bool {
        &self.to_string() == other
    }
}

impl core::fmt::Display for SimpleHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(&self).unwrap().trim_matches('"')
        )
    }
}

/// State root hash of the merkle tree.
pub type StateRootHash = SimpleHash;

/// Serialized key of a node in the state tree.
pub type SerializedTreeNodeKey = Vec<u8>;

/// Serialized value of a node in the state tree.
pub type SerializedTreeNodeValue = Vec<u8>;

/// Hash of a leaf value key in the state tree. This is not the same as a tree node key, but rather
/// a value in the dataset (leaf nodes) and the key that's used to look it up in the state.
pub type StateKeyHash = SimpleHash;

/// Encapsulation of a value (leaf node) key in the state tree, including the state table name and
/// entry key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateKey {
    // TODO(snormore): Make this an enum.
    pub table: String,
    // TODO(snormore): Make this a type.
    pub key: Vec<u8>,
}

impl StateKey {
    pub fn new(table: String, key: Vec<u8>) -> Self {
        Self { table, key }
    }

    pub fn hash<S: SerdeBackend, H: SimpleHasher>(&self) -> StateKeyHash {
        StateKeyHash::build::<H>(S::serialize(&self))
    }
}

// TODO(snormore): Define our own type for this instead of leaking the JMT type.
pub type StateProof<VH> = SparseMerkleProof<VH>;
