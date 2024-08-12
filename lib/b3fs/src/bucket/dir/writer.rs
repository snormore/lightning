use crate::bucket::Bucket;
use crate::directory::{DirectoryHasher, OwnedEntry};

/// A trusted
pub struct DirWriter {
    hasher: DirectoryHasher,
}

impl DirWriter {
    pub fn new(bucket: &Bucket) -> Self {
        todo!()
    }

    pub async fn insert(&mut self, entry: OwnedEntry) {}
}
