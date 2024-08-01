use std::borrow::Borrow;

use atomo::SerdeBackend;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{MerklizedStrategy, StateKey, StateRootHash};

/// Proof of a state value in the state tree.
/// This is a commitment proof that can be used to verify the existence or non-existence of a value
/// in the state tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateProof(ics23::CommitmentProof);

impl StateProof {
    /// Create a new `StateProof` with the given ics23 commitment proof.
    pub fn new(proof: ics23::CommitmentProof) -> Self {
        Self(proof)
    }

    /// Verify the membership of a key-value pair in the state tree.
    /// This is used to verify that a key exists in the state tree and has the given value. It
    /// encapsulates the serialization of the key and value, and relies on the ics23 crate to
    /// verify the proof from there.
    pub fn verify_membership<K, V, M: MerklizedStrategy>(
        &self,
        table: impl AsRef<str>,
        key: impl Borrow<K>,
        value: impl Borrow<V>,
        root: StateRootHash,
    ) -> bool
    where
        K: Serialize,
        V: Serialize,
    {
        let state_key = StateKey::new(table, M::Serde::serialize(&key.borrow()));
        let serialized_key = M::Serde::serialize(&state_key);
        let serialized_value = M::Serde::serialize(value.borrow());
        ics23::verify_membership::<ics23::HostFunctionsManager>(
            &self.0,
            &M::ics23_proof_spec(),
            &root.as_ref().to_vec(),
            &serialized_key,
            serialized_value.as_slice(),
        )
    }

    /// Verify the non-membership of a key in the state tree.
    /// This is used to verify that a key does not exist in the state tree. It encapsulates the
    /// serialization of the key, and relies on the ics23 crate to verify the proof from there.
    pub fn verify_non_membership<K, M: MerklizedStrategy>(
        self,
        table: impl AsRef<str>,
        key: impl Borrow<K>,
        root: StateRootHash,
    ) -> bool
    where
        K: Serialize,
    {
        let state_key = StateKey::new(table, M::Serde::serialize(&key.borrow()));
        let serialized_key = M::Serde::serialize(&state_key);
        ics23::verify_non_membership::<ics23::HostFunctionsManager>(
            &self.0,
            &M::ics23_proof_spec(),
            &root.as_ref().to_vec(),
            &serialized_key,
        )
    }
}

impl From<StateProof> for ics23::CommitmentProof {
    fn from(proof: StateProof) -> Self {
        proof.0
    }
}

impl From<ics23::CommitmentProof> for StateProof {
    fn from(proof: ics23::CommitmentProof) -> Self {
        Self(proof)
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
