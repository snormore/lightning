use atomo::{DefaultSerdeBackend, InMemoryStorage, SerdeBackend, StorageBackendConstructor};
use atomo_rocks::{Options, RocksBackendBuilder};
use merklized::hashers::blake3::Blake3Hasher;
use merklized::hashers::keccak::KeccakHasher;
use merklized::hashers::sha2::Sha256Hasher;
use merklized::strategies::jmt::JmtMerklizedStrategy;
use merklized::{MerklizedAtomoBuilder, MerklizedStrategy, StateRootHash};
use tempfile::tempdir;

#[test]
fn test_atomo_memdb_sha256() {
    init_logger();

    let builder = InMemoryStorage::default();
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, Sha256Hasher>>(builder);
}

#[test]
fn test_atomo_rocksdb_sha256() {
    init_logger();

    let temp_dir = tempdir().unwrap();
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);
    let builder = RocksBackendBuilder::new(temp_dir.path()).with_options(options);
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, Sha256Hasher>>(builder);
}

#[test]
fn test_atomo_memdb_keccak256() {
    init_logger();

    let builder = InMemoryStorage::default();
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, KeccakHasher>>(builder);
}

#[test]
fn test_atomo_rocksdb_keccak256() {
    init_logger();

    let temp_dir = tempdir().unwrap();
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);
    let builder = RocksBackendBuilder::new(temp_dir.path()).with_options(options);
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, KeccakHasher>>(builder);
}

#[test]
fn test_atomo_memdb_blake3() {
    init_logger();

    let builder = InMemoryStorage::default();
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, Blake3Hasher>>(builder);
}

#[test]
fn test_atomo_rocksdb_blake3() {
    init_logger();

    let temp_dir = tempdir().unwrap();
    let mut options = Options::default();
    options.create_if_missing(true);
    options.create_missing_column_families(true);
    let builder = RocksBackendBuilder::new(temp_dir.path()).with_options(options);
    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, Blake3Hasher>>(builder);
}

fn generic_test_atomo<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    M: MerklizedStrategy<Storage = C::Storage, Serde = S>,
>(
    builder: C,
) {
    let mut db = MerklizedAtomoBuilder::<C, S, M>::new(builder)
        .with_table::<String, String>("data")
        .enable_iter("data")
        .with_table::<u8, u8>("other")
        .build()
        .unwrap();
    let reader = db.query();

    // Check state root.
    let initial_state_root = reader.get_state_root().unwrap();
    assert_eq!(
        initial_state_root,
        "5350415253455f4d45524b4c455f504c414345484f4c4445525f484153485f5f"
    );
    let mut old_state_root = initial_state_root;

    // Insert initial data.
    let data_insert_count = 10;
    db.run(|ctx: _| {
        let mut data_table = ctx.get_table::<String, String>("data");

        for i in 1..=data_insert_count {
            data_table.insert(format!("key{i}"), format!("value{i}"));
        }
    });

    // Check data via reader.
    reader.run(|ctx| {
        // Check state root.
        let new_state_root = reader.get_state_root().unwrap();
        assert_ne!(new_state_root, old_state_root);
        assert_ne!(new_state_root, StateRootHash::default());
        old_state_root = new_state_root;

        let data_table = ctx.get_table::<String, String>("data");
        let ctx = M::context(ctx);

        // Check data key count.
        let keys = data_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), data_insert_count);

        // Check data values for each key.
        for i in 1..=data_insert_count {
            assert_eq!(data_table.get(format!("key{i}")), Some(format!("value{i}")));
        }

        // Check existence proofs.
        for i in 1..=data_insert_count {
            // Generate proof.
            let (value, proof) = ctx
                .get_state_proof(
                    "data",
                    M::Serde::serialize::<Vec<u8>>(&format!("key{i}").as_bytes().to_vec()),
                )
                .unwrap();
            assert_eq!(
                value.map(|v| M::Serde::deserialize::<String>(&v.to_vec())),
                Some(format!("value{i}"))
            );

            println!("{}", serde_json::ser::to_string_pretty(&proof).unwrap());

            // Verify proof.
            {
                let proof: ics23::CommitmentProof = proof.clone().into();
                assert!(matches!(
                    proof.proof,
                    Some(ics23::commitment_proof::Proof::Exist(_))
                ));
            }
            assert!(proof.verify_membership::<String, String, M>(
                "data",
                format!("key{i}").to_string(),
                format!("value{i}").to_string(),
                new_state_root,
            ));
        }

        // Check non-existence proof.
        let (value, proof) = ctx
            .get_state_proof("data", S::serialize(&"unknown".to_string()))
            .unwrap();
        assert!(value.is_none());
        {
            let proof: ics23::CommitmentProof = proof.clone().into();
            assert!(matches!(
                proof.proof,
                Some(ics23::commitment_proof::Proof::Nonexist(_))
            ));
        }
        assert!(proof.verify_non_membership::<String, M>(
            "data",
            "unknown".to_string(),
            new_state_root,
        ));
    });

    // Insert more data.
    db.run(|ctx: _| {
        let mut data_table = ctx.get_table::<String, String>("data");

        for i in 1..=data_insert_count {
            data_table.insert(format!("other{i}"), format!("value{i}"));
        }
    });

    // Check state root.
    let new_state_root = reader.get_state_root().unwrap();
    assert_ne!(new_state_root, old_state_root);
    assert_ne!(new_state_root, StateRootHash::default());
    // let old_state_root = new_state_root;

    // Remove some data.
    db.run(|ctx: _| {
        let mut data_table = ctx.get_table::<String, String>("data");

        data_table.remove("key3".to_string());
        data_table.remove("other5".to_string());
        data_table.remove("other9".to_string());
    });

    // Check state root.
    let new_state_root = reader.get_state_root().unwrap();
    // TODO(snormore): Figure out why the state root is not changing after removing data.
    // assert_ne!(new_state_root, old_state_root);
    assert_ne!(new_state_root, StateRootHash::default());

    // Check non-membership proofs for removed data.
    // TODO(snormore): Figure out why these non-existence proof verification are failing. Probably
    // the same reason as the state root not changing above.
    // reader.run(|ctx| {
    //     let ctx = M::context(ctx);

    //     // Check non-existence proof for key3.
    //     let (value, proof) = ctx
    //         .get_state_proof("data", S::serialize(&"key3".to_string()))
    //         .unwrap();
    //     assert!(value.is_none());
    //     assert!(proof.verify_non_membership::<String, M>(
    //         "data",
    //         "key3".to_string(),
    //         new_state_root,
    //     ));

    //     // Check non-existence proof for other5.
    //     let (value, proof) = ctx
    //         .get_state_proof("data", S::serialize(&"other5".to_string()))
    //         .unwrap();
    //     assert!(value.is_none());
    //     assert!(proof.verify_non_membership::<String, M>(
    //         "data",
    //         "other5".to_string(),
    //         new_state_root,
    //     ));

    //     // Check non-existence proof for other9.
    //     let (value, proof) = ctx
    //         .get_state_proof("data", S::serialize(&"other9".to_string()))
    //         .unwrap();
    //     assert!(value.is_none());
    //     assert!(proof.verify_non_membership::<String, M>(
    //         "data",
    //         "other9".to_string(),
    //         new_state_root,
    //     ));
    // });
}

#[allow(dead_code)]
pub fn init_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default())
        .is_test(true)
        .format_timestamp(None)
        .try_init();
}
