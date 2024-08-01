use anyhow::Result;
use atomo::{
    AtomoBuilder,
    DefaultSerdeBackend,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableSelector,
};

use crate::{MerklizedContext, SimpleHasher};

#[cfg(feature = "strategy-jmt")]
pub type DefaultMerklizedStrategy<B, H> =
    crate::strategies::jmt::JmtMerklizedStrategy<B, DefaultSerdeBackend, H>;

#[cfg(feature = "hasher-blake3")]
pub type DefaultMerklizedStrategyWithHasherBlake3<B> =
    DefaultMerklizedStrategy<B, crate::hashers::blake3::Blake3Hasher>;

#[cfg(feature = "hasher-keccak")]
pub type DefaultMerklizedStrategyWithHasherKeccak<B> =
    DefaultMerklizedStrategy<B, crate::hashers::keccak::KeccakHasher>;

#[cfg(feature = "hasher-sha2")]
pub type DefaultMerklizedStrategyWithHasherSha256<B> =
    DefaultMerklizedStrategy<B, crate::hashers::sha2::Sha256Hasher>;

/// A trait for a merklized strategy that can be used to build a `[atomo::Atomo]` instance, and
/// provide a merklized execution context.
pub trait MerklizedStrategy {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;

    /// Initialize and return an atomo instance for this strategy.
    fn atomo<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, Self::Serde>>;

    /// Initialize and return a new execution context using this strategy.
    fn context<'a>(
        ctx: &'a TableSelector<Self::Storage, Self::Serde>,
    ) -> Box<dyn MerklizedContext<'a, Self::Storage, Self::Serde, Self::Hasher> + 'a>
    where
        Self::Hasher: SimpleHasher + 'a;

    /// Return the ics23 proof spec for this strategy.
    fn ics23_proof_spec() -> ics23::ProofSpec;
}
