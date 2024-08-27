use atomo::{SerdeBackend, StorageBackend, TableSelector};

use super::table::AtomoDatabaseTable;
use crate::database::{DatabaseRunContext, DatabaseTable};

pub struct AtomoDatabaseRunContext<B: StorageBackend, S: SerdeBackend> {
    inner: TableSelector<B, S>,
}

impl<B: StorageBackend, S: SerdeBackend> AtomoDatabaseRunContext<B, S> {
    pub fn new(inner: TableSelector<B, S>) -> Self {
        Self { inner }
    }
}

impl<B: StorageBackend, S: SerdeBackend> DatabaseRunContext for AtomoDatabaseRunContext<B, S> {
    type Storage = B;
    type Serde = S;

    fn get_table<T: DatabaseTable>(&self, name: impl AsRef<str>) -> T {
        AtomoDatabaseTable::new(self.inner.get_table(name))
    }
}
