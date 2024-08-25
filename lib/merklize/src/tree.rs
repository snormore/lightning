use atomo::AtomoBuilder;

use crate::{StateTreeBuilder, StateTreeConfig, StateTreeReader, StateTreeWriter};

pub trait StateTree: Sized {
    type Config: StateTreeConfig;

    type Builder: StateTreeBuilder<Self>;
    type Writer: StateTreeWriter<Self>;
    type Reader: StateTreeReader<Self::Config>;

    /// Returns a new state tree.
    fn new() -> Self;

    /// Returns a builder for the state tree.
    fn builder(
        self,
        builder: AtomoBuilder<
            <Self::Config as StateTreeConfig>::StorageBuilder,
            <Self::Config as StateTreeConfig>::Serde,
        >,
    ) -> Self::Builder;
}
