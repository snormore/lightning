use fleek_blake3::tree::HashTreeBuilder;

use crate::bucket::Bucket;

pub struct FileWriter {
    hasher: HashTreeBuilder,
}

impl FileWriter {
    pub fn new(bucket: &Bucket) -> Self {
        todo!()
    }

    pub async fn write(&mut self, bytes: &[u8]) {}
}
