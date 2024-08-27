use atomo::{Atomo, QueryPerm, SerdeBackend, StorageBackend};

use super::super::DatabaseReader;
use super::context::AtomoDatabaseRunContext;

pub struct AtomoDatabaseReader<B: StorageBackend, S: SerdeBackend> {
    inner: Atomo<QueryPerm, B, S>,
}

impl<B: StorageBackend, S: SerdeBackend> Clone for AtomoDatabaseReader<B, S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<B: StorageBackend, S: SerdeBackend> AtomoDatabaseReader<B, S> {
    pub fn new(inner: Atomo<QueryPerm, B, S>) -> Self {
        Self { inner }
    }
}

impl<B: StorageBackend, S: SerdeBackend> DatabaseReader for AtomoDatabaseReader<B, S> {
    type Storage = B;
    type Serde = S;

    type RunContext = AtomoDatabaseRunContext<B, S>;

    fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&Self::RunContext) -> R,
    {
        self.inner
            .run(|ctx| query(AtomoDatabaseRunContext::new(ctx)))
    }
}
