#![feature(test)]
extern crate test;

mod utils;

use atomo::{SerdeBackend, StorageBackendConstructor};
use merklized::hashers::keccak::KeccakHasher;
use merklized::{DefaultMerklizedStrategy, MerklizedAtomoBuilder, MerklizedStrategy};
use rand::Rng;
use tempfile::tempdir;
use test::Bencher;
use utils::{rocksdb_builder, DATA_COUNT_COMPLEX, DATA_COUNT_MEDIUM, DATA_COUNT_SIMPLE};

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_keccak256_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_blake3_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_sha256_simple(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_SIMPLE,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_keccak256_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_blake3_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_sha256_medium(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_MEDIUM,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_keccak256_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_blake3_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

#[bench]
fn bench_generic_generate_proof_rocksdb_jmt_sha256_complex(b: &mut Bencher) {
    let temp_dir = tempdir().unwrap();
    generic_bench_generate_proof::<_, DefaultMerklizedStrategy<_, KeccakHasher>>(
        b,
        rocksdb_builder(&temp_dir),
        DATA_COUNT_COMPLEX,
    );
}

fn generic_bench_generate_proof<C: StorageBackendConstructor, M: MerklizedStrategy>(
    b: &mut Bencher,
    builder: C,
    data_count: usize,
) where
    M: MerklizedStrategy<Storage = C::Storage>,
{
    let mut db = MerklizedAtomoBuilder::<C, M::Serde, M>::new(builder)
        .with_table::<String, String>("data")
        .build()
        .unwrap();

    db.run(|ctx| {
        let mut data_table = ctx.get_table::<String, String>("data");

        for i in 1..=data_count {
            data_table.insert(format!("key{i}"), format!("value{i}"));
        }
    });

    b.iter(|| {
        db.query().run(|ctx| {
            let ctx = M::context(ctx);
            let i = rand::thread_rng().gen_range(1..=data_count);
            let (value, _proof) = ctx
                .get_state_proof("data", M::Serde::serialize(&format!("key{i}")))
                .unwrap();
            assert_eq!(value, Some(M::Serde::serialize(&format!("value{i}"))));
        });
    })
}
