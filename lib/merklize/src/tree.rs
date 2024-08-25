use atomo::{SerdeBackend, StorageBackendConstructor};

use crate::{SimpleHasher, StateProof, StateTreeBuilder, StateTreeReader, StateTreeWriter};

pub trait StateTree: Sized {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
    type Proof: StateProof;

    type Builder: StateTreeBuilder<Self>;
    type Writer: StateTreeWriter<Self>;
    type Reader: StateTreeReader<Self>;

    /// Returns a builder for the state tree.
    fn builder(&self) -> Self::Builder;

    /// Returns a writer for the state tree. There can only be one of these. Calling it more than
    /// once should not even compile.
    // TODO(snormore): Make sure there can only be one of these.
    fn writer(&self) -> Self::Writer;

    /// Returns a reader for the state tree. This can be called many times and will return a new
    /// clone each time.
    fn reader(&self) -> Self::Reader;
}
