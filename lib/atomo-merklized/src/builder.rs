use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{AtomoBuilder, SerdeBackend, StorageBackendConstructor, UpdatePerm};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{MerklizedAtomo, MerklizedStrategy};

type AtomoResult<P, B, S, M, E> = Result<MerklizedAtomo<P, B, S, M>, E>;

pub struct MerklizedAtomoBuilder<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    X: MerklizedStrategy<Storage = C::Storage, Serde = S>,
> {
    inner: AtomoBuilder<C, S>,
    _phantom: PhantomData<X>,
}

impl<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    X: MerklizedStrategy<Storage = C::Storage, Serde = S>,
> MerklizedAtomoBuilder<C, S, X>
{
    /// Create a new builder with the given storage backend constructor.
    pub fn new(constructor: C) -> Self {
        Self {
            inner: AtomoBuilder::new(constructor),
            _phantom: PhantomData,
        }
    }

    /// Open a new table with the given name and key-value type.
    /// This is a pass-through to `[atomo::AtomoBuilder::with_table]`.
    #[inline]
    pub fn with_table<K, V>(self, table: impl ToString) -> Self
    where
        K: Hash + Eq + Serialize + DeserializeOwned + Any,
        V: Serialize + DeserializeOwned + Any,
    {
        Self {
            inner: self.inner.with_table::<K, V>(table),
            ..self
        }
    }

    /// Enable key iteration on the table with the given name.
    /// This is a pass-through to `[atomo::AtomoBuilder::enable_iter]`.
    #[inline]
    pub fn enable_iter(self, table: impl ToString) -> Self {
        Self {
            inner: self.inner.enable_iter(&table.to_string()),
            ..self
        }
    }

    /// Build and return a writer for the state tree.
    pub fn build(self) -> AtomoResult<UpdatePerm, C::Storage, S, X, C::Error> {
        // TODO(snormore): Fix this unwrap.
        let atomo = X::build(self.inner).unwrap();
        Ok(MerklizedAtomo::<_, C::Storage, S, X>::new(atomo))
    }
}
