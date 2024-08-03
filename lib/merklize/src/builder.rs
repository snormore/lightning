use std::any::Any;
use std::hash::Hash;
use std::marker::PhantomData;

use atomo::{AtomoBuilder, SerdeBackend, StorageBackendConstructor, UpdatePerm};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{MerklizeProvider, MerklizedAtomo};

type AtomoResult<P, B, S, M, E> = Result<MerklizedAtomo<P, B, S, M>, E>;

/// A builder for a merklize atomo instance, wrapping `[atomo::AtomoBuilder]`, to provide similar
/// functionality, but for building `[merklize::MerklizedAtomo]` instances instead of
/// `[atomo::Atomo]` instances.
///
/// Most methods are passthroughs to the inner atomo builder, but the build method is augmented to
/// first build the atomo instance and then wrap it in a merklize atomo instance.
pub struct MerklizedAtomoBuilder<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    M: MerklizeProvider<Storage = C::Storage, Serde = S>,
> {
    inner: AtomoBuilder<C, S>,
    _phantom: PhantomData<M>,
}

impl<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    M: MerklizeProvider<Storage = C::Storage, Serde = S>,
> MerklizedAtomoBuilder<C, S, M>
{
    /// Create a new builder with the given storage backend constructor.
    pub fn new(constructor: C) -> Self {
        Self {
            inner: AtomoBuilder::new(constructor),
            _phantom: PhantomData,
        }
    }

    /// Open a new table with the given name and key-value type.
    /// This is a direct passthrough to the inner atomo instance.
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
    /// This is a direct passthrough to the inner atomo instance.
    #[inline]
    pub fn enable_iter(self, table: impl ToString) -> Self {
        Self {
            inner: self.inner.enable_iter(&table.to_string()),
            ..self
        }
    }

    /// Build and return a writer for the state tree. This is a passthrough to the inner atomo
    /// instance, but augments by first building with the merklize provider, so that the state tree
    /// tables can be initialized as well.
    pub fn build(self) -> AtomoResult<UpdatePerm, C::Storage, S, M, C::Error> {
        let atomo = M::atomo(self.inner).unwrap();
        Ok(MerklizedAtomo::<_, C::Storage, S, M>::new(atomo))
    }
}
