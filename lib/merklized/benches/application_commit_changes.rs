#![feature(test)]
extern crate test;

mod application_utils;
mod utils;

use std::path::PathBuf;
use std::sync::Arc;

use atomo::UpdatePerm;
use atomo_rocks::Options;
use futures::executor::block_on;
use lightning_application::env::Env;
use lightning_application::storage::{AtomoStorage, AtomoStorageBuilder};
use tempfile::{tempdir, TempDir};
use test::Bencher;
use tokio::sync::Mutex;

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let env = Arc::new(Mutex::new(create_merklized_rocksdb_env(&temp_dir)));
    b.iter(|| block_on(application_utils::execute_txn_and_query_simple(env.clone())))
}

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let env = Arc::new(Mutex::new(create_merklized_rocksdb_env(&temp_dir)));
    b.iter(|| {
        block_on(application_utils::execute_txn_and_query_medium(
            env.clone(),
            50,
        ))
    })
}

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let env = Arc::new(Mutex::new(create_merklized_rocksdb_env(&temp_dir)));
    b.iter(|| {
        block_on(application_utils::execute_txn_and_query_complex(
            env.clone(),
            50,
        ))
    })
}

fn create_merklized_rocksdb_env(temp_dir: &TempDir) -> Env<UpdatePerm, AtomoStorage> {
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);

    let storage = AtomoStorageBuilder::new(Some(temp_dir.path())).with_options(options);

    application_utils::create_merklized_app_env(storage, PathBuf::from(temp_dir.path()))
}
