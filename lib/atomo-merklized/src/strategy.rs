use anyhow::Result;
use atomo::{AtomoBuilder, SerdeBackend, StorageBackend, StorageBackendConstructor, TableSelector};

use crate::{MerklizedContext, SimpleHasher};

pub trait MerklizedStrategy {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;

    /// Initialize and return an atomo instance for this strategy.
    fn build<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, Self::Serde>>;

    /// Initialize and return a new execution context using this strategy.
    fn context<'a>(
        ctx: &'a TableSelector<Self::Storage, Self::Serde>,
    ) -> Box<dyn MerklizedContext<'a, Self::Storage, Self::Serde, Self::Hasher> + 'a>
    where
        Self::Hasher: SimpleHasher + 'a;
}
