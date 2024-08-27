use atomo::{Atomo, SerdeBackend, StorageBackend, UpdatePerm};

use super::context::AtomoDatabaseRunContext;
use super::reader::AtomoDatabaseReader;
use crate::database::{DatabaseReader, DatabaseRunContext, DatabaseWriter};

pub struct AtomoDatabaseWriter<B: StorageBackend, S: SerdeBackend> {
    inner: Atomo<UpdatePerm, B, S>,
}

impl<B: StorageBackend, S: SerdeBackend> DatabaseWriter for AtomoDatabaseWriter<B, S> {
    type Storage = B;
    type Serde = S;

    type Error = anyhow::Error;

    fn reader<R: DatabaseReader>(&self) -> R {
        AtomoDatabaseReader::new(self.inner.clone())
    }

    fn run<C: DatabaseRunContext, F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut C) -> R,
    {
        self.inner
            .run(|ctx| mutation(&mut AtomoDatabaseRunContext::new(ctx)))
    }
}
