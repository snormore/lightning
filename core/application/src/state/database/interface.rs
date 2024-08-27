use std::borrow::Borrow;
use std::fmt::Debug;

use atomo::{KeyIterator, SerdeBackend, StorageBackend, StorageBackendConstructor};
use serde::Serialize;

pub trait DatabaseBuilder: Clone {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Error: Debug;
    type Database: DatabaseWriter;

    fn build(&self) -> Result<Self::Database, Self::Error>;
}

pub trait DatabaseWriter: Sized {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Error: Debug;

    fn reader<R: DatabaseReader>(&self) -> R;

    fn run<C: DatabaseRunContext, F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut C) -> R;
}

pub trait DatabaseReader: Sized + Clone {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;

    fn run<C: DatabaseRunContext, F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut C) -> R;
}

pub trait DatabaseTable: Sized {
    type Key: Serialize;
    type Value: Serialize;

    /// Insert a new `key` and `value` pair into the table.
    fn insert(&mut self, key: impl Borrow<Self::Key>, value: impl Borrow<Self::Value>);

    /// Remove the given key from the table.
    fn remove(&mut self, key: impl Borrow<Self::Key>);

    /// Returns the value associated with the provided key. If the key doesn't exits in the table
    /// [`None`] is returned.
    fn get(&self, key: impl Borrow<Self::Key>) -> Option<Self::Value>;

    /// Returns `true` if the key exists in the table.
    fn contains_key(&self, key: impl Borrow<Self::Key>) -> bool;

    /// Returns an iterator of the keys in this table.
    // TODO(snormore): Make this a generic iterator type.
    fn keys(&self) -> KeyIterator<Self::Key>;
}

pub trait DatabaseRunContext: Sized {
    // TODO(snormore): Is this needed?
    type Storage: StorageBackend;
    type Serde: SerdeBackend;

    fn get_table<T: DatabaseTable>(&self, name: impl AsRef<str>) -> T;
}
