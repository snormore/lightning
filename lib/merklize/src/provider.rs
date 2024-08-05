use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};

use crate::{MerklizeContext, SimpleHasher, StateProof};

/// A trait for a merklize provider that can be used to build a `[atomo::Atomo]` instance, and
/// provide a merklize execution context.
pub trait MerklizeProvider {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
    type Proof: StateProof;

    /// Augment the provided atomo builder with the necessary tables for the merklize provider.
    fn with_tables<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> AtomoBuilder<C, Self::Serde>;

    /// Initialize and return a new execution context using this provider.
    fn context<'a>(
        ctx: &'a TableSelector<Self::Storage, Self::Serde>,
    ) -> Box<dyn MerklizeContext<'a, Self::Storage, Self::Serde, Self::Hasher, Self::Proof> + 'a>
    where
        Self::Hasher: SimpleHasher + 'a;
}
