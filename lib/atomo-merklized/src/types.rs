use atomo::SerdeBackend;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{SimpleHash, SimpleHasher};

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
    pub key: Vec<u8>,
}

impl StateKey {
    /// Create a new `StateKey` with the given table name and key.
    pub fn new(table: impl AsRef<str>, key: Vec<u8>) -> Self {
        Self {
            table: table.as_ref().to_string(),
            key,
        }
    }

    /// Build and return a hash for the state key.
    pub fn hash<S: SerdeBackend, H: SimpleHasher>(&self) -> StateKeyHash {
        StateKeyHash::build::<H>(S::serialize(&self))
    }
}

// TODO(snormore): Should we have an enum for the different types of proofs?

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProof(ics23::CommitmentProof);

impl From<ics23::CommitmentProof> for StateProof {
    fn from(proof: ics23::CommitmentProof) -> Self {
        Self(proof)
    }
}

impl From<StateProof> for ics23::CommitmentProof {
    fn from(proof: StateProof) -> Self {
        proof.0
    }
}

impl JsonSchema for StateProof {
    fn schema_name() -> String {
        "StateProof".to_string()
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed(concat!(module_path!(), "::StateProof"))
    }

    fn json_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let key = StateProof(ics23::CommitmentProof::default());

        schemars::schema_for_value!(key).schema.into()
    }
}
