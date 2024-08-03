use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use atomo::batch::Operation;
use atomo::{SerdeBackend, StorageBackend, TableId, TableSelector};
use fxhash::FxHashMap;
use jmt::storage::{HasPreimage, LeafNode, Node, NodeKey, TreeReader};
use jmt::{KeyHash, OwnedValue, Version};
use lru::LruCache;
use tracing::{trace, trace_span};

use super::hasher::SimpleHasherWrapper;
use super::provider::{KEYS_TABLE_NAME, NODES_TABLE_NAME};
use crate::strategies::jmt::ics23::ics23_proof_spec;
use crate::{MerklizedContext, SimpleHasher, StateKey, StateProof, StateRootHash};

type SharedTableRef<'a, K, V, B, S> = Arc<Mutex<atomo::TableRef<'a, K, V, B, S>>>;

/// A merklize context that can be used to read and update tables of data, wrapping an
/// `[atomo::TableSelector]` instance to provide similar functionality, but with additional
/// merklize state tree features.
pub struct JmtMerklizedContext<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> {
    ctx: &'a TableSelector<B, S>,
    table_name_by_id: FxHashMap<TableId, String>,
    nodes_table: SharedTableRef<'a, NodeKey, Node, B, S>,
    keys_table: SharedTableRef<'a, KeyHash, StateKey, B, S>,
    keys_cache: Arc<Mutex<LruCache<KeyHash, StateKey>>>,
    _phantom: PhantomData<H>,
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> JmtMerklizedContext<'a, B, S, H> {
    /// Create a new merklize context for the given table selector, initializing state tree tables
    /// and other necessary data for the context functionality.
    pub fn new(ctx: &'a TableSelector<B, S>) -> Self {
        let tables = ctx.tables();

        let nodes_table = ctx.get_table(NODES_TABLE_NAME);
        let keys_table = ctx.get_table(KEYS_TABLE_NAME);

        let mut table_id_by_name = FxHashMap::default();
        for (i, table) in tables.iter().enumerate() {
            let table_id: TableId = i.try_into().unwrap();
            let table_name = table.name.to_string();
            table_id_by_name.insert(table_name, table_id);
        }

        let table_name_by_id = table_id_by_name
            .clone()
            .into_iter()
            .map(|(k, v)| (v, k))
            .collect::<FxHashMap<TableId, String>>();

        Self {
            ctx,
            table_name_by_id,
            nodes_table: Arc::new(Mutex::new(nodes_table)),
            keys_table: Arc::new(Mutex::new(keys_table)),
            keys_cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(512).unwrap()))),
            _phantom: PhantomData,
        }
    }

    /// Get the state key for the given key hash, if it is present in the keys table. If the key is
    /// not found in the keys table, it will return `None`. The result is also cached in an LRU
    /// cache to avoid localized, repeated lookups.
    fn get_key(&self, key_hash: KeyHash) -> Option<StateKey> {
        let mut keys_cache = self.keys_cache.lock().unwrap();

        if let Some(state_key) = keys_cache.get(&key_hash) {
            return Some(state_key.clone());
        }

        let state_key = self.keys_table.lock().unwrap().get(key_hash);
        if let Some(state_key) = state_key.clone() {
            keys_cache.put(key_hash, state_key.clone());
        }

        state_key
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> MerklizedContext<'a, B, S, H>
    for JmtMerklizedContext<'a, B, S, H>
{
    /// Get the state root hash of the state tree.
    fn get_state_root(&self) -> Result<StateRootHash> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        tree.get_root_hash(0).map(|hash| hash.0.into())
    }

    /// Get an existence proof for the given key hash, if it is present in the state tree, or
    /// non-existence proof if it is not present. The proof will include the value if it exists, and
    /// the proof is returned as an `[ics23::CommitmentProof]`.
    fn get_state_proof(
        &self,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<(Option<Vec<u8>>, StateProof)> {
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<S, H>();
        trace!(?key_hash, ?state_key, "get_state_proof");

        let (value, proof) = tree.get_with_ics23_proof(
            S::serialize(&state_key),
            0,
            ics23_proof_spec(H::ICS23_HASH_OP),
        )?;

        Ok((value, proof.into()))
    }

    /// Apply the state tree changes based on the state changes in the atomo batch. This will update
    /// the state tree to reflect the changes in the atomo batch. It reads data from the state tree,
    /// so an execution context is needed to ensure consistency.
    fn apply_state_tree_changes(&mut self) -> Result<()> {
        let span = trace_span!("apply_state_tree_changes");
        let _enter = span.enter();

        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<H>>::new(self);

        // Build a jmt value set (batch) from the atomo batch.
        let mut value_set: Vec<(jmt::KeyHash, Option<jmt::OwnedValue>)> = Default::default();
        {
            let span = trace_span!("build_value_set");
            let _enter = span.enter();

            let batch = self.ctx.batch();
            for (table_id, changes) in batch.into_raw().iter().enumerate() {
                let table_id: TableId = table_id.try_into()?;
                let table_name = self
                    .table_name_by_id
                    .get(&table_id)
                    .ok_or(anyhow!("Table with index {} not found", table_id))?
                    .as_str();
                for (key, operation) in changes.iter() {
                    let state_key = StateKey::new(table_name, key.to_vec());
                    let key_hash = jmt::KeyHash(state_key.hash::<S, H>().into());

                    match operation {
                        Operation::Remove => {
                            // let span = trace_span!("remove_key_value");
                            // let _enter = span.enter();

                            value_set.push((key_hash, None));

                            // Remove it from the keys table.
                            trace!(?key_hash, ?state_key, "removing key");
                            self.keys_table.lock().unwrap().remove(key_hash);
                        },
                        Operation::Insert(value) => {
                            // let span = trace_span!("insert_key_value");
                            // let _enter = span.enter();

                            value_set.push((key_hash, Some(value.to_vec())));

                            // Insert it into the keys table.
                            trace!(?key_hash, ?state_key, "inserting key");
                            self.keys_table
                                .lock()
                                .unwrap()
                                .insert(key_hash, state_key.clone());
                        },
                    }
                }
            }
        }

        // Apply the jmt value set (batch) to the tree.
        let tree_batch = {
            let span = trace_span!("put_value_set");
            let _enter = span.enter();

            let (_new_root_hash, tree_batch) = tree.put_value_set(value_set.clone(), 0).unwrap();
            tree_batch
        };

        // Remove stale nodes.
        {
            let span = trace_span!("remove_stale_nodes");
            let _enter = span.enter();

            for stale_node in tree_batch.stale_node_index_batch {
                self.nodes_table.lock().unwrap().remove(stale_node.node_key);
            }
        }

        // Insert new nodes.
        {
            let span = trace_span!("insert_new_nodes");
            let _enter = span.enter();

            for (node_key, node) in tree_batch.node_batch.nodes() {
                self.nodes_table.lock().unwrap().insert(node_key, node);
            }
        }

        Ok(())
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> TreeReader
    for JmtMerklizedContext<'a, B, S, H>
{
    /// Get the node for the given node key, if it is present in the tree.
    fn get_node_option(&self, node_key: &NodeKey) -> Result<Option<Node>> {
        let value = self.nodes_table.lock().unwrap().get(node_key);
        trace!(?node_key, ?value, "get_node_option");
        match value {
            Some(value) => Ok(Some(value)),
            None => {
                if node_key.nibble_path().is_empty() {
                    // If the nibble path is empty, it's a root node and we should return a null
                    // node instead of None.
                    Ok(Some(Node::Null))
                } else {
                    Ok(None)
                }
            },
        }
    }

    /// Get the leftmost leaf node in the tree.
    /// This is not currently used, so it returns an error.
    fn get_rightmost_leaf(&self) -> Result<Option<(NodeKey, LeafNode)>> {
        unreachable!("Not currently used")
    }

    /// Get the state value for the given key hash, if it is present in the tree.
    fn get_value_option(
        &self,
        _max_version: Version,
        key_hash: KeyHash,
    ) -> Result<Option<OwnedValue>> {
        let state_key = self.get_key(key_hash);
        let value = if let Some(state_key) = state_key {
            self.ctx.get_raw_value(state_key.table, &state_key.key)
        } else {
            None
        };
        trace!(?key_hash, ?value, "get_value_option");
        Ok(value)
    }
}

impl<'a, B: StorageBackend, S: SerdeBackend, H: SimpleHasher> HasPreimage
    for JmtMerklizedContext<'a, B, S, H>
{
    /// Gets the preimage of a key hash, if it is present in the tree.
    fn preimage(&self, key_hash: KeyHash) -> Result<Option<Vec<u8>>> {
        let state_key = self.get_key(key_hash);
        trace!(?key_hash, ?state_key, "preimage");
        Ok(state_key.map(|key| S::serialize(&key)))
    }
}

#[cfg(test)]
mod tests {
    use atomo::{
        Atomo,
        AtomoBuilder,
        DefaultSerdeBackend,
        InMemoryStorage,
        StorageBackendConstructor,
        UpdatePerm,
    };

    use super::*;
    use crate::hashers::sha2::Sha256Hasher;
    use crate::DefaultMerklizeProvider;

    fn build_atomo<C: StorageBackendConstructor, S: SerdeBackend>(
        builder: C,
    ) -> Atomo<UpdatePerm, C::Storage, S> {
        AtomoBuilder::<_, S>::new(builder)
            .with_table::<String, String>("data")
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .unwrap()
    }

    #[test]
    fn test_apply_state_tree_changes_with_updates() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;

        let mut db = build_atomo::<_, S>(InMemoryStorage::default());

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 0); // nodes
            assert_eq!(storage.keys(2).len(), 0); // keys
        }

        // Insert a value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key1".to_string(), "value1".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 1); // nodes
            assert_eq!(storage.keys(2).len(), 1); // keys
        }

        // Insert another value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key2".to_string(), "value2".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 1); // nodes
            assert_eq!(storage.keys(2).len(), 2); // keys
        }
    }

    #[test]
    fn test_apply_state_tree_changes_with_no_changes() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;

        let mut db = build_atomo::<_, S>(InMemoryStorage::default());

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 0); // nodes
            assert_eq!(storage.keys(2).len(), 0); // keys
        }

        // Open run context and apply state tree changes, but don't make any state changes before.
        db.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 0); // nodes
            assert_eq!(storage.keys(2).len(), 0); // keys
        }

        // Insert another value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key2".to_string(), "value2".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check storage.
        {
            let storage = db.get_storage_backend_unsafe();
            assert_eq!(storage.keys(1).len(), 1); // nodes
            assert_eq!(storage.keys(2).len(), 1); // keys
        }
    }

    #[test]
    fn test_get_state_root_with_empty_state() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;

        let db = build_atomo::<_, S>(InMemoryStorage::default());
        let query = db.query();

        let state_root = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_root()
                .unwrap()
        });
        assert_eq!(
            state_root,
            "5350415253455f4d45524b4c455f504c414345484f4c4445525f484153485f5f"
        );
    }

    #[test]
    fn test_get_state_root_with_updates() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;

        let mut db = build_atomo::<_, S>(InMemoryStorage::default());
        let query = db.query();

        // Check the state root hash.
        let empty_state_root = "5350415253455f4d45524b4c455f504c414345484f4c4445525f484153485f5f";
        let initial_state_root = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_root()
                .unwrap()
        });
        assert_ne!(initial_state_root, StateRootHash::default());
        assert_eq!(initial_state_root, empty_state_root);

        // Insert a value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key1".to_string(), "value1".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check the state root hash.
        let new_state_root = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_root()
                .unwrap()
        });
        assert_ne!(new_state_root, StateRootHash::default());
        assert_ne!(initial_state_root, new_state_root);
        let old_state_root = new_state_root;

        // Insert another value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key2".to_string(), "value2".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Check the state root hash.
        let new_state_root = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_root()
                .unwrap()
        });
        assert_ne!(new_state_root, StateRootHash::default());
        assert_ne!(old_state_root, new_state_root);
    }

    #[test]
    fn test_get_state_proof_of_membership() {
        type S = DefaultSerdeBackend;
        type H = Sha256Hasher;
        type M = DefaultMerklizeProvider<InMemoryStorage, H>;

        let mut db = build_atomo::<_, S>(InMemoryStorage::default());
        let query = db.query();

        // Get a proof of non-membership with empty state, should fail.
        let res = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_proof("data", S::serialize(&"key1".to_string()))
        });
        assert!(res.is_err());
        assert_eq!(
            res.err().unwrap().to_string(),
            "Cannot manufacture nonexistence proof by exclusion for the empty tree"
        );

        // Insert a value.
        db.run(|ctx| {
            let mut table = ctx.get_table::<String, String>("data");

            table.insert("key1".to_string(), "value1".to_string());

            JmtMerklizedContext::<_, _, H>::new(ctx)
                .apply_state_tree_changes()
                .unwrap();
        });

        // Get state root for proof verification.
        let state_root = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_root()
                .unwrap()
        });

        // Get and verify proof of membership.
        let (value, proof) = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_proof("data", S::serialize(&"key1".to_string()))
                .unwrap()
        });
        assert_eq!(value, Some(S::serialize(&"value1".to_string())));
        {
            let proof: ics23::CommitmentProof = proof.clone().into();
            assert!(matches!(
                proof.proof,
                Some(ics23::commitment_proof::Proof::Exist(_))
            ));
        }
        assert!(proof.verify_membership::<String, String, M>(
            "data",
            "key1".to_string(),
            "value1".to_string(),
            state_root
        ));

        // Get and verify proof of non-membership of unknown key.
        let (value, proof) = query.run(|ctx| {
            JmtMerklizedContext::<_, _, H>::new(ctx)
                .get_state_proof("data", S::serialize(&"unknown".to_string()))
                .unwrap()
        });
        assert_eq!(value, None);
        {
            let proof: ics23::CommitmentProof = proof.clone().into();
            assert!(matches!(
                proof.proof,
                Some(ics23::commitment_proof::Proof::Nonexist(_))
            ));
        }
        assert!(proof.verify_non_membership::<String, M>(
            "data",
            "unknown".to_string(),
            state_root
        ));
    }
}
