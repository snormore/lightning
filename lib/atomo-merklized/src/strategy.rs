use std::any::Any;
use std::borrow::Borrow;
use std::hash::Hash;

use anyhow::Result;
use atomo::batch::VerticalBatch;
use atomo::{SerdeBackend, StorageBackend, TableIndex};
use fxhash::FxHashMap;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{SerializedTreeNodeKey, SerializedTreeNodeValue, StateRootHash, StateTable};

/// A strategy for a merklized atomo describing the configuration and architecture of the
/// database-backed merkle state tree.
pub trait MerklizedStrategy {
    /// Returns the root hash of the state tree.
    fn get_root<B: StorageBackend, S: SerdeBackend>(
        tree_table: &atomo::TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
    ) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state. If the key exists in
    /// the state, the value and an existence proof is returned. If the key does not exist in the
    /// state, `[None]` is returned along with a non-existent proof.
    fn get_proof<K, V, B: StorageBackend, S: SerdeBackend>(
        tree_table: &atomo::TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table: StateTable,
        key: impl Borrow<K>,
        value: Option<V>,
    ) -> Result<(Option<V>, Vec<u8>)>
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any;
    // TODO(snormore): Return a proof type instead of a `Vec<u8>`, or something standard like an
    // ics23 proof.

    /// Applies the changes in the given batch to the state tree by computing updated or removed
    /// nodes, to be committed with same state updates.
    fn apply_changes<B: StorageBackend, S: SerdeBackend>(
        tree_table: &mut atomo::TableRef<SerializedTreeNodeKey, SerializedTreeNodeValue, B, S>,
        table_name_by_id: FxHashMap<TableIndex, String>,
        batch: VerticalBatch,
    ) -> Result<()>;
}
