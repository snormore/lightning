use atomo::{KeyIterator, SerdeBackend, StorageBackend, TableSelector};

use super::super::DatabaseRunContext;
use crate::database::DatabaseTable;

pub struct AtomoDatabaseRunContext<'ctx, B: StorageBackend, S: SerdeBackend> {
    inner: &'ctx TableSelector<B, S>,
}

impl<'ctx, B: StorageBackend, S: SerdeBackend> AtomoDatabaseRunContext<'ctx, B, S> {
    pub fn new(inner: &'ctx TableSelector<B, S>) -> Self {
        Self { inner }
    }
}

impl<'ctx, B: StorageBackend, S: SerdeBackend> DatabaseRunContext
    for AtomoDatabaseRunContext<'ctx, B, S>
{
    type Storage = B;
    type Serde = S;

    fn insert<T: DatabaseTable>(&mut self, key: T::Key, value: T::Value) {
        self.inner.get_table(T::NAME).insert(key, value)
    }

    fn remove<T: DatabaseTable>(&mut self, key: T::Key) {
        self.inner
            .get_table::<T::Key, T::Value>(T::NAME)
            .remove(key)
    }

    fn get<T: DatabaseTable>(&self, key: T::Key) -> Option<T::Value> {
        self.inner.get_table::<T::Key, T::Value>(T::NAME).get(key)
    }

    fn contains_key<T: DatabaseTable>(&self, key: T::Key) -> bool {
        self.inner
            .get_table::<T::Key, T::Value>(T::NAME)
            .contains_key(key)
    }

    fn keys<T: DatabaseTable>(&self) -> KeyIterator<T::Key> {
        self.inner.get_table::<T::Key, T::Value>(T::NAME).keys()
    }
}
