use atomo::SerdeBackend;
use jmt::SimpleHasher;

use crate::MerklizedStrategy;

pub trait MerklizedLayout: Clone + Send + Sync + 'static {
    type SerdeBackend: SerdeBackend + Send + Sync;
    type Strategy: MerklizedStrategy;
    type KeyHasher: SimpleHasher;
    type ValueHasher: SimpleHasher;
}
