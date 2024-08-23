use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use anyhow::{ensure, Result};
use atomo::batch::Operation;
use atomo::{
    Atomo,
    AtomoBuilder,
    InMemoryStorage,
    QueryPerm,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableId,
    TableSelector,
};
use fxhash::FxHashMap;
use tracing::{trace, trace_span};

use super::adapter::Adapter;
use super::hasher::SimpleHasherWrapper;
use super::proof::ics23_proof_spec;
use super::tree::{KEYS_TABLE_NAME, NODES_TABLE_NAME, TREE_VERSION};
use super::{JmtStateProof, JmtStateTree};
use crate::{
    SimpleHasher,
    StateKey,
    StateRootHash,
    StateTree,
    StateTreeBuilder,
    StateTreeReader,
    StateTreeWriter,
    VerifyStateTreeError,
};

#[derive(Clone)]
pub struct JmtStateTreeReader<T: StateTree> {
    _tree: PhantomData<T>,
}

impl<T: StateTree> JmtStateTreeReader<T> {
    pub fn new() -> Self {
        Self { _tree: PhantomData }
    }
}

impl<T: StateTree> Default for JmtStateTreeReader<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: StateTree> StateTreeReader<T> for JmtStateTreeReader<T>
where
    T: StateTree<Proof = JmtStateProof>,
{
    /// Get the state root hash of the state tree.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_root(
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
    ) -> Result<StateRootHash> {
        let span = trace_span!("get_state_root");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let keys_table = Arc::new(Mutex::new(ctx.get_table(KEYS_TABLE_NAME)));

        let adapter = Adapter::new(ctx, nodes_table, keys_table);
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<T::Hasher>>::new(&adapter);

        tree.get_root_hash(TREE_VERSION).map(|hash| hash.0.into())
    }

    /// Get an existence proof for the given key hash, if it is present in the state tree, or
    /// non-existence proof if it is not present.
    /// Since we need to read the state, a table selector execution context is needed for
    /// consistency.
    fn get_state_proof(
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<T::Proof> {
        let span = trace_span!("get_state_proof");
        let _enter = span.enter();

        let nodes_table = Arc::new(Mutex::new(ctx.get_table(NODES_TABLE_NAME)));
        let keys_table = Arc::new(Mutex::new(ctx.get_table(KEYS_TABLE_NAME)));

        let adapter = Adapter::new(ctx, nodes_table, keys_table);
        let tree = jmt::JellyfishMerkleTree::<_, SimpleHasherWrapper<T::Hasher>>::new(&adapter);

        let state_key = StateKey::new(table, serialized_key);
        let key_hash = state_key.hash::<T::Serde, T::Hasher>();

        trace!(?key_hash, ?state_key, "get_state_proof");

        let (_value, proof) = tree.get_with_ics23_proof(
            T::Serde::serialize(&state_key),
            TREE_VERSION,
            ics23_proof_spec(T::Hasher::ICS23_HASH_OP),
        )?;

        let proof: JmtStateProof = proof.into();

        Ok(proof)
    }

    /// Verify that the state in the given atomo database instance, when used to build a new,
    /// temporary state tree from scratch, matches the stored state tree root hash.
    fn verify_state_tree_unsafe(
        db: &mut Atomo<
            QueryPerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("verify_state_tree");
        let _enter = span.enter();

        // Build batch of all state data.
        let tables = db.tables();
        let mut batch = HashMap::new();
        for (i, table) in tables.clone().into_iter().enumerate() {
            let tid = i as u8;

            let mut changes = Vec::new();
            for (key, value) in db.get_storage_backend_unsafe().get_all(tid) {
                changes.push((key, Operation::Insert(value)));
            }
            batch.insert(table, changes.into_iter());
        }

        // Build a new, temporary state tree from the batch.
        type TempStateTree<T> =
            JmtStateTree<InMemoryStorage, <T as StateTree>::Serde, <T as StateTree>::Hasher>;
        let builder = AtomoBuilder::<_, T::Serde>::new(InMemoryStorage::default());
        let mut tmp_db = <<TempStateTree<T> as StateTree>::Builder as StateTreeBuilder<
            TempStateTree<T>,
        >>::register_tables(builder)
        .build()?;
        tmp_db.run(|ctx| <TempStateTree<T> as StateTree>::Writer::update_state_tree(ctx, batch))?;

        // Get and return the state root hash from the temporary state tree.
        let tmp_state_root = tmp_db
            .query()
            .run(|ctx| <TempStateTree<T> as StateTree>::Reader::get_state_root(ctx))?;

        // Check that the state root hash matches the stored state root hash.
        let stored_state_root = db.query().run(|ctx| Self::get_state_root(ctx))?;
        ensure!(
            tmp_state_root == stored_state_root,
            VerifyStateTreeError::StateRootMismatch(stored_state_root, tmp_state_root)
        );

        Ok(())
    }

    fn is_empty_state_tree_unsafe(
        db: &mut Atomo<
            atomo::QueryPerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
        >,
    ) -> Result<bool> {
        let span = trace_span!("is_empty_state_tree");
        let _enter = span.enter();

        let tables = db.tables();
        let table_id_by_name = tables
            .iter()
            .enumerate()
            .map(|(tid, table)| (table.clone(), tid as TableId))
            .collect::<FxHashMap<_, _>>();

        let nodes_table_id = *table_id_by_name.get(NODES_TABLE_NAME).unwrap();

        let storage = db.get_storage_backend_unsafe();

        // TODO(snormore): This should use an iterator to avoid loading all keys into memory. We
        // only need to see if there is at least one key in each table, so `.next()` on an
        // iterator should be sufficient.
        Ok(storage.keys(nodes_table_id).len() == 0)
    }
}
