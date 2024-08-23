use atomo::{AtomoBuilder, SerdeBackend, StorageBackendConstructor};

use crate::{SimpleHasher, StateProof, StateTreeBuilder, StateTreeReader, StateTreeWriter};

pub trait StateTree: Sized {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
    type Proof: StateProof;

    type Builder: StateTreeBuilder<Self>;
    type Writer: StateTreeWriter<Self>;
    type Reader: StateTreeReader<Self>;

    /// Returns a new state tree.
    fn new() -> Self;

    /// Returns a builder for the state tree.
    fn builder(self, builder: AtomoBuilder<Self::StorageBuilder, Self::Serde>) -> Self::Builder;
}
