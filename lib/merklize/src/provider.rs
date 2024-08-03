use anyhow::Result;
use atomo::{
    AtomoBuilder,
    DefaultSerdeBackend,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableSelector,
};

use crate::{MerklizeContext, SimpleHasher};

#[cfg(feature = "provider-jmt")]
pub type DefaultMerklizeProvider<B, H> =
    crate::providers::jmt::JmtMerklizeProvider<B, DefaultSerdeBackend, H>;

#[cfg(feature = "hasher-blake3")]
pub type DefaultMerklizeProviderWithHasherBlake3<B> =
    DefaultMerklizeProvider<B, crate::hashers::blake3::Blake3Hasher>;

#[cfg(feature = "hasher-keccak")]
pub type DefaultMerklizeProviderWithHasherKeccak<B> =
    DefaultMerklizeProvider<B, crate::hashers::keccak::KeccakHasher>;

#[cfg(feature = "hasher-sha2")]
pub type DefaultMerklizeProviderWithHasherSha256<B> =
    DefaultMerklizeProvider<B, crate::hashers::sha2::Sha256Hasher>;

/// A trait for a merklize provider that can be used to build a `[atomo::Atomo]` instance, and
/// provide a merklize execution context.
pub trait MerklizeProvider {
    type Storage: StorageBackend;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;

    /// Initialize and return an atomo instance for this provider.
    fn atomo<C: StorageBackendConstructor>(
        builder: AtomoBuilder<C, Self::Serde>,
    ) -> Result<atomo::Atomo<atomo::UpdatePerm, C::Storage, Self::Serde>>;

    /// Initialize and return a new execution context using this provider.
    fn context<'a>(
        ctx: &'a TableSelector<Self::Storage, Self::Serde>,
    ) -> Box<dyn MerklizeContext<'a, Self::Storage, Self::Serde, Self::Hasher> + 'a>
    where
        Self::Hasher: SimpleHasher + 'a;

    /// Return the ics23 proof spec for this provider.
    fn ics23_proof_spec() -> ics23::ProofSpec;
}
