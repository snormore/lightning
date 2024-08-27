use std::fmt::Debug;

use atomo::{SerdeBackend, StorageBackend, StorageBackendConstructor};

pub trait TreeBuilder: Clone {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Error: Debug;
    type Tree: TreeWriter;

    fn build(&self) -> Result<Self::Tree, Self::Error>;
}

pub trait TreeWriter: Sized {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Error: Debug;

    fn reader<R: TreeReader>(&self) -> R;

    fn run<C: TreeRunContext, F, R>(&mut self, mutation: F) -> R
    where
        F: FnOnce(&mut C) -> R;
}

pub trait TreeReader: Sized + Clone {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;

    fn run<C: TreeRunContext, F, R>(&self, query: F) -> R
    where
        F: FnOnce(&mut C) -> R;
}

pub trait TreeRunContext: Sized {
    // TODO(snormore): Is this needed?
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
}
