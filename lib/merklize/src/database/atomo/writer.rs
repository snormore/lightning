use atomo::{Atomo, SerdeBackend, StorageBackend, UpdatePerm};

use super::super::{DatabaseReader, DatabaseRunContext, DatabaseWriter};
use super::context::AtomoDatabaseRunContext;
use super::reader::AtomoDatabaseReader;

pub struct AtomoDatabaseWriter<B: StorageBackend, S: SerdeBackend> {
    inner: Atomo<UpdatePerm, B, S>,
}

impl<B: StorageBackend, S: SerdeBackend> DatabaseWriter for AtomoDatabaseWriter<B, S> {
    type Storage = B;
    type Serde = S;

    type Error = anyhow::Error;
    type Reader = AtomoDatabaseReader<B, S>;
    type RunContext = AtomoDatabaseRunContext<B, S>;

    fn reader(&self) -> Self::Reader {
        AtomoDatabaseReader::new(self.inner.query())
    }

    fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&Self::RunContext) -> R,
    {
        self.inner.run(|ctx| {
            let ctx = AtomoDatabaseRunContext::new(&ctx);
            mutation(ctx)
        })
    }
}
