use std::borrow::Borrow;
use std::fmt::Debug;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::tree::StateRootHash;

/// Proof of a state value in the state tree.
/// This is a commitment proof that can be used to verify the existence or non-existence of a value
/// in the state tree.
pub trait StateProof: Debug + Clone + Serialize + DeserializeOwned + Send + Sync {
    /// Verify the membership of a key-value pair in the state tree.
    fn verify_membership<K, V>(
        &self,
        table: impl AsRef<str>,
        key: impl Borrow<K>,
        value: impl Borrow<V>,
        root: StateRootHash,
    ) -> Result<()>
    where
        K: Serialize,
        V: Serialize;

    /// Verify the non-membership of a key in the state tree.
    fn verify_non_membership<K>(
        self,
        table: impl AsRef<str>,
        key: impl Borrow<K>,
        root: StateRootHash,
    ) -> Result<()>
    where
        K: Serialize;
}
