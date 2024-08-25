use std::marker::PhantomData;

use atomo::{SerdeBackend, StorageBackendConstructor};

use crate::{SimpleHasher, StateTreeConfig};

#[derive(Debug, Clone)]
pub struct MptStateTreeConfig<B: StorageBackendConstructor, S: SerdeBackend, H: SimpleHasher> {
    _storage: PhantomData<B>,
    _serde: PhantomData<S>,
    _hasher: PhantomData<H>,
}

impl<B, S, H> StateTreeConfig for MptStateTreeConfig<B, S, H>
where
    B: StorageBackendConstructor + Clone,
    S: SerdeBackend + Clone,
    H: SimpleHasher + Clone,
{
    type StorageBuilder = B;
    type Serde = S;
    type Hasher = H;
}
