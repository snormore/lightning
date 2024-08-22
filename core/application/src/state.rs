use std::marker::PhantomData;

use atomo::{Atomo, DefaultSerdeBackend, QueryPerm, TableSelector, UpdatePerm};
use lightning_interfaces::SyncQueryRunnerInterface;
use merklize::MerklizeProvider;

use crate::query_runner::QueryRunner;
use crate::state_executor::StateExecutor;
use crate::storage::AtomoStorage;
use crate::table::Backend;

pub struct ApplicationState<P, StateTree: MerklizeProvider> {
    db: Atomo<P, StateTree::Storage, StateTree::Serde>,
    _tree: PhantomData<StateTree>,
}

impl<P, StateTree: MerklizeProvider> ApplicationState<P, StateTree> {
    pub fn new(atomo: AtomoBuilder<P, StateTree::Storage, StateTree::Serde>) -> Self {
        let db = atomo.build();
        Self {
            db,
            _tree: PhantomData,
        }
    }
}

impl<StateTree> ApplicationState<UpdatePerm, StateTree>
where
    StateTree: MerklizeProvider<Storage = AtomoStorage, Serde = DefaultSerdeBackend>,
{
    pub fn executor<B: Backend>(backend: B) -> StateExecutor<B> {
        StateExecutor::new(backend)
    }

    pub fn query(&self) -> ApplicationState<QueryPerm, StateTree> {
        ApplicationState::new(self.db.query())
    }

    pub fn query_runner(&self) -> QueryRunner {
        QueryRunner::new(self.db.query())
    }

    pub fn get_storage_backend_unsafe(&mut self) -> &StateTree::Storage {
        self.db.get_storage_backend_unsafe()
    }

    pub fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut TableSelector<StateTree::Storage, StateTree::Serde>) -> R,
    {
        self.db.run(|ctx| {
            let result = mutation(ctx);

            // TODO(snormore): Fix this expect/panic.
            StateTree::update_state_tree_from_context(ctx).expect("Failed to update state tree");

            result
        })
    }
}

impl<StateTree> ApplicationState<QueryPerm, StateTree>
where
    StateTree: MerklizeProvider<Storage = AtomoStorage, Serde = DefaultSerdeBackend>,
{
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut TableSelector<StateTree::Storage, StateTree::Serde>) -> R,
    {
        self.db.run(query)
    }
}
