mod context;
mod executor;
mod query;
mod tables;

use anyhow::{anyhow, Result};
use atomo::{
    Atomo,
    AtomoBuilder,
    DefaultSerdeBackend,
    StorageBackendConstructor,
    TableSelector,
    UpdatePerm,
};
use context::StateContext;
use executor::StateExecutor;
use lightning_interfaces::SyncQueryRunnerInterface;
use merklize::hashers::keccak::KeccakHasher;
use merklize::providers::mpt::MptMerklizeProvider;
use merklize::MerklizeProvider;
pub use query::QueryRunner;
pub use tables::ApplicationStateTables;

use crate::storage::AtomoStorage;

/// A canonical application state tree implementation.
pub type ApplicationMerklizeProvider =
    MptMerklizeProvider<AtomoStorage, DefaultSerdeBackend, KeccakHasher>;

/// The shared application state accumulates by executing transactions.
pub struct ApplicationState<StateTree: MerklizeProvider> {
    db: Atomo<UpdatePerm, StateTree::Storage, StateTree::Serde>,
}

impl<StateTree> ApplicationState<StateTree>
where
    StateTree: MerklizeProvider<Storage = AtomoStorage, Serde = DefaultSerdeBackend>,
{
    /// Creates a new application state.
    pub(crate) fn new(db: Atomo<UpdatePerm, AtomoStorage, DefaultSerdeBackend>) -> Self {
        Self { db }
    }

    /// Registers the application and state tree tables, and builds the atomo database.
    pub fn build<C>(atomo: AtomoBuilder<C, DefaultSerdeBackend>) -> Result<Self>
    where
        C: StorageBackendConstructor<Storage = AtomoStorage>,
    {
        let atomo = ApplicationStateTables::register(atomo);
        let atomo = StateTree::with_tables(atomo);

        let db = atomo
            .build()
            .map_err(|e| anyhow!("Failed to build atomo: {:?}", e))?;

        Ok(Self::new(db))
    }

    /// Returns a reader for the application state.
    pub fn query(&self) -> QueryRunner {
        QueryRunner::new(self.db.query())
    }

    /// Returns a mutable reference to the atomo storage backend.
    ///
    /// This is unsafe because it allows modifying the state tree without going through the
    /// executor, which can lead to inconsistent state across nodes.
    pub fn get_storage_backend_unsafe(&mut self) -> &AtomoStorage {
        self.db.get_storage_backend_unsafe()
    }

    /// Returns a state executor that handles transaction execution logic, reading and modifying the
    /// state.
    pub fn executor(
        ctx: &mut TableSelector<AtomoStorage, DefaultSerdeBackend>,
    ) -> StateExecutor<StateContext<AtomoStorage, DefaultSerdeBackend>> {
        StateExecutor::new(StateContext {
            table_selector: ctx,
        })
    }

    /// Runs a mutation on the state.
    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut TableSelector<AtomoStorage, DefaultSerdeBackend>) -> R,
    {
        self.db.run(|ctx| {
            let result = mutation(ctx);

            // TODO(snormore): Fix this unwrap/panic.
            StateTree::update_state_tree_from_context(ctx).unwrap();

            result
        })
    }
}
