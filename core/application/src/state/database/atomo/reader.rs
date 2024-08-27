use atomo::{Atomo, QueryPerm, SerdeBackend, StorageBackend};

use super::context::AtomoDatabaseRunContext;
use crate::database::{DatabaseReader, DatabaseRunContext};

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

impl<B: StorageBackend, S: SerdeBackend> DatabaseReader for AtomoDatabaseReader<B, S> {
    type Storage = B;
    type Serde = S;

    fn run<C: DatabaseRunContext, F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut C) -> R,
    {
        self.inner
            .run(|ctx| query(&mut AtomoDatabaseRunContext::new(ctx)))
    }
}
