use std::collections::HashMap;

use anyhow::Result;
use atomo::batch::Operation;
use atomo::{Atomo, StorageBackend, StorageBackendConstructor, TableId, TableSelector, UpdatePerm};
use fxhash::FxHashMap;
use tracing::trace_span;

use crate::StateTree;

/// A trait for a merklize provider used to maintain and interact with the state tree.
///
/// ## Examples
///
/// ```rust
#[doc = include_str!("../examples/jmt-sha256.rs")]
/// ```
pub trait StateTreeWriter<T: StateTree> {
    /// Applies the changes in the given batch of updates to the state tree.
    ///
    /// This method uses an atomo execution context, so it is safe to use concurrently.
    ///
    /// Arguments:
    /// - `ctx`: The atomo execution context that will be used to apply the changes.
    /// - `batch`: The batch of pending changes to apply to the state tree.
    fn update_state_tree<I>(
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
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
        db: &mut Atomo<
            UpdatePerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
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
        ctx: &TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
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

        Self::update_state_tree(ctx, batch)
    }

    /// Clears existing state tree and rebuilds it from scratch. This does not delete or modify any
    /// of the state data, just the tree structure and tables related to it. The tree is then
    /// rebuilt by applying all of the state data in the atomo context to the new tree.
    ///
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    ///
    /// Arguments:
    /// - `db`: The atomo database instance to use for clearing and rebuilding the state tree.
    fn clear_and_rebuild_state_tree_unsafe(
        db: &mut Atomo<
            UpdatePerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            T::Serde,
        >,
    ) -> Result<()> {
        let span = trace_span!("clear_and_rebuild_state_tree");
        let _enter = span.enter();

        Self::clear_state_tree_unsafe(db)?;

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

        db.run(|ctx| Self::update_state_tree(ctx, batch))
    }
}
