use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;

use atomo::{KeyIterator, SerdeBackend, StorageBackend, StorageBackendConstructor};
use serde::de::DeserializeOwned;
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

    type Reader: DatabaseReader;
    type RunContext: DatabaseRunContext<Storage = Self::Storage, Serde = Self::Serde>;

    fn reader(&self) -> Self::Reader;

    fn tx(&self) -> Self::RunContext;

    // fn tx_mut(&self) -> &mut Self::RunContext;

    fn run<F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&Self::RunContext) -> R;
}

pub trait DatabaseReader: Sized + Clone {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;

    type RunContext: DatabaseRunContext<Storage = Self::Storage, Serde = Self::Serde>;

    fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(&Self::RunContext) -> R;
}

pub trait DatabaseTableKey: Hash + Eq + Serialize + DeserializeOwned + Any {}

pub trait DatabaseTableValue: Serialize + DeserializeOwned + Any {}

pub trait DatabaseTable: Sized {
    const NAME: &'static str;

    type Key: DatabaseTableKey;
    type Value: DatabaseTableValue;
}

pub trait DatabaseRunContext: Sized {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;

    fn insert<T: DatabaseTable>(&mut self, key: T::Key, value: T::Value);

    /// Remove the given key from the table.
    fn remove<T: DatabaseTable>(&mut self, key: T::Key);

    /// Returns the value associated with the provided key. If the key doesn't exits in the table
    /// [`None`] is returned.
    fn get<T: DatabaseTable>(&self, key: T::Key) -> Option<T::Value>;

    /// Returns `true` if the key exists in the table.
    fn contains_key<T: DatabaseTable>(&self, key: T::Key) -> bool;

    /// Returns an iterator of the keys in this table.
    // TODO(snormore): Make this a generic iterator type.
    fn keys<T: DatabaseTable>(&self) -> KeyIterator<T::Key>;
}
