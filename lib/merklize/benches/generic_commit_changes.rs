#![feature(test)]
extern crate test;

mod generic_utils;

use atomo::{AtomoBuilder, StorageBackendConstructor};
use generic_utils::{rocksdb_builder, DATA_COUNT_COMPLEX, DATA_COUNT_MEDIUM, DATA_COUNT_SIMPLE};
use merklize::hashers::keccak::KeccakHasher;
use merklize::{DefaultMerklizeProvider, MerklizeProvider, MerklizedAtomoBuilder};
use tempfile::tempdir;
use test::Bencher;

#[bench]
fn bench_generic_commit_changes_rocksdb_baseline_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_baseline_bench_commit_changes(b, rocksdb_builder(&temp_dir), DATA_COUNT_SIMPLE);
}

#[bench]
fn bench_generic_commit_changes_rocksdb_baseline_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_baseline_bench_commit_changes(b, rocksdb_builder(&temp_dir), DATA_COUNT_MEDIUM);
}

#[bench]
fn bench_generic_commit_changes_rocksdb_baseline_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_baseline_bench_commit_changes(b, rocksdb_builder(&temp_dir), DATA_COUNT_COMPLEX);
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_keccak256_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_blake3_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_sha256_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_keccak256_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_blake3_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_sha256_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_keccak256_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_blake3_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

#[bench]
fn bench_generic_commit_changes_rocksdb_jmt_sha256_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_merklize_bench_commit_changes::<_, DefaultMerklizeProvider<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

fn generic_baseline_bench_commit_changes<C: StorageBackendConstructor>(
    b: &mut Bencher,
    builder: C,
    data_count: usize,
) {
    let mut db = AtomoBuilder::<C>::new(builder)
        .with_table::<String, String>("data")
        .build()
        .unwrap();

    b.iter(|| {
        db.run(|ctx| {
            let mut data_table = ctx.get_table::<String, String>("data");

            for i in 1..=data_count {
                data_table.insert(format!("key{i}"), format!("value{i}"));
            }
        });
    })
}

fn generic_merklize_bench_commit_changes<C: StorageBackendConstructor, M>(
    b: &mut Bencher,
    builder: C,
    data_count: usize,
) where
    M: MerklizeProvider<Storage = C::Storage>,
{
    let mut db = MerklizedAtomoBuilder::<C, M::Serde, M>::new(builder)
        .with_table::<String, String>("data")
        .build()
        .unwrap();

    b.iter(|| {
        db.run(|ctx| {
            let mut data_table = ctx.get_table::<String, String>("data");

            for i in 1..=data_count {
                data_table.insert(format!("key{i}"), format!("value{i}"));
            }
        });
    })
}
