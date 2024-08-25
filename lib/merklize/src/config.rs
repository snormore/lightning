use atomo::{SerdeBackend, StorageBackendConstructor};

use crate::SimpleHasher;

pub trait StateTreeConfig: Clone {
    type StorageBuilder: StorageBackendConstructor;
    type Serde: SerdeBackend;
    type Hasher: SimpleHasher;
}
