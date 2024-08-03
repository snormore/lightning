#![feature(test)]
extern crate test;

mod application_utils;

use application_utils::{
    create_rocksdb_env,
    new_complex_block,
    new_medium_block,
    new_simple_block,
    DummyPutter,
};
use futures::executor::block_on;
use tempfile::tempdir;
use test::Bencher;

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env(&temp_dir);
    let (block, _stake_amount, _eth_addresses) = new_simple_block();
    b.iter(|| block_on(env.run(block.clone(), || DummyPutter {})))
}

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env(&temp_dir);
    let (block, _stake_amount, _eth_addresses) = new_medium_block();
    b.iter(|| block_on(env.run(block.clone(), || DummyPutter {})))
}

#[bench]
fn bench_application_commit_changes_rocksdb_merklized_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env(&temp_dir);
    let (block, _stake_amount, _eth_addresses, _node_public_keys) = new_complex_block();
    b.iter(|| block_on(env.run(block.clone(), || DummyPutter {})))
}
