use atomo::AtomoBuilder;

use crate::StateTree;

/// A trait for working with the atomo builder to prepare it for managing a state tree.
pub trait StateTreeBuilder<T: StateTree> {
    /// Augment the provided atomo builder with the necessary tables for the state tree.
    ///
    /// Arguments:
    /// - `builder`: The atomo builder to augment.
    fn register_tables(
        builder: AtomoBuilder<T::StorageBuilder, T::Serde>,
    ) -> AtomoBuilder<T::StorageBuilder, T::Serde>;
}
