use atomo::SerdeBackend;

use crate::MerklizedStrategy;

pub trait MerklizedLayout: Clone + Send + Sync + 'static {
    type SerdeBackend: SerdeBackend + Send + Sync;
    type Strategy: MerklizedStrategy;
    // TODO(snormore): This is leaking `jmt::SimpleHasher`.
    type KeyHasher: jmt::SimpleHasher;
    type ValueHasher: jmt::SimpleHasher;
}
