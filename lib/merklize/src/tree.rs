use std::collections::HashMap;

use anyhow::Result;
use atomo::batch::Operation;
use atomo::{
    Atomo,
    AtomoBuilder,
    QueryPerm,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableId,
    TableSelector,
    UpdatePerm,
};
use fxhash::FxHashMap;
use tracing::trace_span;

use crate::{SimpleHasher, StateProof, StateRootHash};

pub trait StateTree: Sized {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;

    type Proof: StateProof;

    /// Returns a new state tree.
    fn new() -> Self;

    fn register_tables(
        &self,
        builder: AtomoBuilder<Self::StorageBuilder, Self::Serde>,
    ) -> AtomoBuilder<Self::StorageBuilder, Self::Serde>;

    /// Applies the changes in the given batch of updates to the state tree.
    ///
    /// This method uses an atomo execution context, so it is safe to use concurrently.
    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to apply the changes.
    /// - `batch`: The batch of pending changes to apply to the state tree.
    fn update_state_tree<I>(
        &self,
        ctx: &TableSelector<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
        batch: HashMap<String, I>,
    ) -> Result<()>
    where
        I: Iterator<Item = (Box<[u8]>, Operation)>;

    /// Clears the existing state tree data. This does not delete or modify any of the state data,
    /// just the tree structure and tables related to it.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to use for clearing the state tree.
    fn clear_state_tree_unsafe(
        &self,
        db: &mut Atomo<
            UpdatePerm,
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<()>;

    /// Applies the pending changes in the given context to the state tree.
    /// This is an implementation that makes use of the `update_state_tree` method, passing it the
    /// batch of pending changes from the context.
    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to get the pending changes and apply
    ///   them to the state tree.
    fn update_state_tree_from_context(
        &self,
        ctx: &TableSelector<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("update_state_tree_from_context");
        let _enter = span.enter();

        let mut table_name_by_id = FxHashMap::default();
        for (i, table) in ctx.tables().into_iter().enumerate() {
            table_name_by_id.insert(i as TableId, table);
        }

        // Build batch of pending changes from the context.
        let batch = ctx
            .batch()
            .into_raw()
            .into_iter()
            .enumerate()
            .map(|(tid, changes)| {
                let table = table_name_by_id.get(&(tid as TableId)).unwrap().clone();
                let changes = changes.into_iter().map(|(k, v)| (k.clone(), v.clone()));
                (table, changes)
            })
            .collect();

        self.update_state_tree(ctx, batch)
    }

    /// Clears existing state tree and rebuilds it from scratch. This does not delete or modify any
    /// of the state data, just the tree structure and tables related to it. The tree is rebuilt by
    /// applying all of the state data in the atomo context to the new tree.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to use for clearing and rebuilding the state tree.
    fn clear_and_rebuild_state_tree_unsafe(
        &self,
        db: &mut Atomo<
            atomo::UpdatePerm,
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("clear_and_rebuild_state_tree");
        let _enter = span.enter();

        self.clear_state_tree_unsafe(db)?;

        // Build batch of all state data.
        let mut batch = HashMap::new();
        for (i, table) in db.tables().into_iter().enumerate() {
            let tid = i as u8;

            let mut changes = Vec::new();
            for (key, value) in db.get_storage_backend_unsafe().get_all(tid) {
                changes.push((key, Operation::Insert(value)));
            }
            batch.insert(table, changes.into_iter());
        }

        db.run(|ctx| self.update_state_tree(ctx, batch))
    }

    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to get the root hash of the state
    ///   tree.
    fn get_state_root(
        &self,
        ctx: &TableSelector<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<StateRootHash>;

    /// Generates and returns a merkle proof for the given key in the state.
    ///
    /// This method uses an atomo execution context, so it is safe to use concurrently.
    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to generate the proof.
    /// - `table`: The name of the table to generate the proof for.
    /// - `serialized_key`: The serialized key to generate the proof for.
    fn get_state_proof(
        &self,
        ctx: &TableSelector<
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
        table: &str,
        serialized_key: Vec<u8>,
    ) -> Result<Self::Proof>;

    /// Verifies that the state in the given atomo database instance, when used to build a
    /// new, temporary state tree from scratch, matches the stored state tree root hash.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to verify.
    fn verify_state_tree_unsafe(
        &self,
        db: &mut Atomo<
            QueryPerm,
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<()>;

    /// Returns whether the state tree is empty.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to check.
    // TODO(snormore): Can we do this without mut self?
    fn is_empty_state_tree_unsafe(
        &self,
        db: &mut Atomo<
            QueryPerm,
            <Self::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
    ) -> Result<bool>;
}
