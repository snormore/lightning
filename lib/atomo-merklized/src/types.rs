use std::hash::Hash;

use atomo::SerdeBackend;
use digest::generic_array::GenericArray;
use digest::{Digest, OutputSizeUser};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A simple hash type that wraps a 32-byte array.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, JsonSchema)]
pub struct SimpleHash([u8; 32]);

impl serde::Serialize for SimpleHash {
    /// Serialize the hash as a hex string.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        hex::serde::serialize(self.0, serializer)
    }
}

impl<'de> serde::Deserialize<'de> for SimpleHash {
    /// Deserialize the hash from a hex string.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        hex::serde::deserialize(deserializer).map(SimpleHash)
    }
}

impl SimpleHash {
    pub fn build<H: SimpleHasher>(key: impl AsRef<[u8]>) -> Self {
        Self(H::hash(key.as_ref()))
    }
}

impl From<[u8; 32]> for SimpleHash {
    /// Create a new `SimpleHash` from a 32-byte array.
    fn from(hash: [u8; 32]) -> Self {
        Self(hash)
    }
}

impl From<SimpleHash> for [u8; 32] {
    /// Convert a `SimpleHash` to a 32-byte array.
    fn from(hash: SimpleHash) -> Self {
        hash.0
    }
}

impl AsRef<[u8]> for SimpleHash {
    /// Get a reference to the hash byte array.
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<&str> for SimpleHash {
    fn eq(&self, other: &&str) -> bool {
        &self.to_string() == other
    }
}

impl PartialEq<SimpleHash> for &str {
    fn eq(&self, other: &SimpleHash) -> bool {
        self == &other.to_string()
    }
}

impl core::fmt::Display for SimpleHash {
    /// Display the hash as a hex string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(serde_json::to_string(&self).unwrap().trim_matches('"'))
    }
}

impl core::fmt::Debug for SimpleHash {
    /// Represent the hash as a hex JSON string in debug output.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(serde_json::to_string(&self).unwrap().as_str())
    }
}

/// State root hash of the merkle tree.
// TODO(snormore): Should this just be an `[ics23::CommitmentRoot]`?
pub type StateRootHash = SimpleHash;

/// Hash of a leaf value key in the state tree. This is not the same as a tree node key, but rather
/// a value in the dataset (leaf nodes) and the key that's used to look it up in the state.
pub type StateKeyHash = SimpleHash;

/// Encapsulation of a value (leaf node) key in the state tree, including the state table name and
/// entry key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateKey {
    pub table: String,
    key: Vec<u8>,
}

impl StateKey {
    /// Create a new `StateKey` with the given table name and key.
    pub fn new(table: String, key: Vec<u8>) -> Self {
        Self { table, key }
    }

    /// Build and return a hash for the state key.
    pub fn hash<S: SerdeBackend, H: SimpleHasher>(&self) -> StateKeyHash {
        StateKeyHash::build::<H>(S::serialize(&self))
    }
}

/// A table in the state database.
#[derive(Debug, Clone)]
pub struct StateTable {
    name: String,
}

impl StateTable {
    /// Create a new `StateTable` with the given name.
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_string(),
        }
    }

    /// Get the name of the table.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Build and return a state key for the given serialized key.
    pub fn key(&self, key: Vec<u8>) -> StateKey {
        StateKey::new(self.name.clone(), key)
    }
}

pub trait SimpleHasher: Sized {
    fn new() -> Self;

    fn update(&mut self, data: &[u8]);

    fn finalize(self) -> [u8; 32];

    fn hash(data: impl AsRef<[u8]>) -> [u8; 32] {
        let mut hasher = Self::new();
        hasher.update(data.as_ref());
        hasher.finalize()
    }
}

impl<T: Digest> SimpleHasher for T
where
    [u8; 32]: From<GenericArray<u8, <T as OutputSizeUser>::OutputSize>>,
{
    fn new() -> Self {
        <T as Digest>::new()
    }

    fn update(&mut self, data: &[u8]) {
        self.update(data)
    }

    fn finalize(self) -> [u8; 32] {
        self.finalize().into()
    }
}
