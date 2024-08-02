use atomo_rocks::{Options, RocksBackendBuilder};
use tempfile::TempDir;

#[allow(dead_code)]
pub const DATA_COUNT_SIMPLE: usize = 10;

#[allow(dead_code)]
pub const DATA_COUNT_MEDIUM: usize = 100;

#[allow(dead_code)]
pub const DATA_COUNT_COMPLEX: usize = 1000;

#[allow(dead_code)]
pub fn rocksdb_builder(temp_dir: &TempDir) -> RocksBackendBuilder {
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);

    RocksBackendBuilder::new(temp_dir.path()).with_options(options)
}
