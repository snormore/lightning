use atomo::{DefaultSerdeBackend, InMemoryStorage, SerdeBackend, StorageBackendConstructor};
use atomo_merklized::{MerklizedAtomoBuilder, MerklizedStrategy, StateTable};
use atomo_merklized_jmt::JmtMerklizedStrategy;
use jmt::proof::{INTERNAL_DOMAIN_SEPARATOR, LEAF_DOMAIN_SEPARATOR};

/// This is originally defined in the jmt crate but is not publicly exported, so we redefine it here
/// until it is.
const SPARSE_MERKLE_PLACEHOLDER_HASH: [u8; 32] = *b"SPARSE_MERKLE_PLACEHOLDER_HASH__";

#[test]
fn test_atomo() {
    init_logger();

    fn ics23_spec(hash_op: ics23::HashOp) -> ics23::ProofSpec {
        ics23::ProofSpec {
            leaf_spec: Some(ics23::LeafOp {
                hash: hash_op.into(),
                prehash_key: hash_op.into(),
                prehash_value: hash_op.into(),
                length: ics23::LengthOp::NoPrefix.into(),
                prefix: LEAF_DOMAIN_SEPARATOR.to_vec(),
            }),
            inner_spec: Some(ics23::InnerSpec {
                hash: hash_op.into(),
                child_order: vec![0, 1],
                min_prefix_length: INTERNAL_DOMAIN_SEPARATOR.len() as i32,
                max_prefix_length: INTERNAL_DOMAIN_SEPARATOR.len() as i32,
                child_size: 32,
                empty_child: SPARSE_MERKLE_PLACEHOLDER_HASH.to_vec(),
            }),
            min_depth: 0,
            max_depth: 64,
            prehash_key_before_comparison: true,
        }
    }

    generic_test_atomo::<_, DefaultSerdeBackend, JmtMerklizedStrategy<_, _, sha2::Sha256>>(
        InMemoryStorage::default(),
        ics23_spec(ics23::HashOp::Sha256),
    );

    // generic_test_atomo::<_, DefaultSerdeBackend, blake3::Hasher, JmtMerklizedStrategy<_, _, _>>(
    //     InMemoryStorage::default(),
    //     ics23_spec(ics23::HashOp::Blake3),
    // );

    // generic_test_atomo::<_, DefaultSerdeBackend, KeccakHasher, JmtMerklizedStrategy<_, _, _>>(
    //     InMemoryStorage::default(),
    //     ics23_spec(ics23::HashOp::Keccak256),
    // );
}

fn generic_test_atomo<
    C: StorageBackendConstructor,
    S: SerdeBackend,
    X: MerklizedStrategy<Storage = C::Storage, Serde = S>,
>(
    builder: C,
    ics23_spec: ics23::ProofSpec,
) {
    let mut db = MerklizedAtomoBuilder::<C, S, X>::new(builder)
        .with_table::<String, String>("data")
        .enable_iter("data")
        .with_table::<u8, u8>("other")
        .build()
        .unwrap();
    let reader = db.query();

    let data_insert_count = 10;

    // Insert initial data.
    {
        db.run(|ctx: _| {
            let mut data_table = ctx.get_table::<String, String>("data");

            for i in 1..=data_insert_count {
                data_table.insert(format!("key{i}"), format!("value{i}"));
            }
        });

        // Check state root.
        let root_hash = reader.get_state_root().unwrap();
        assert_eq!(
            root_hash,
            "23b5ec5bdc76df17e4e522abff1772f642b87553c229ba96bc6487c83c726d04"
        );

        // Check data via reader.
        reader.run(|ctx| {
            let data_table = ctx.get_table::<String, String>("data");
            let ctx = X::context(ctx);

            // Check data key count.
            let keys = data_table.keys().collect::<Vec<_>>();
            assert_eq!(keys.len(), data_insert_count);

            // Check data values for each key.
            for i in 1..=data_insert_count {
                assert_eq!(data_table.get(format!("key{i}")), Some(format!("value{i}")));
            }

            // Check state root.
            let root_hash = ctx.get_state_root().unwrap();
            assert_eq!(
                root_hash,
                "23b5ec5bdc76df17e4e522abff1772f642b87553c229ba96bc6487c83c726d04"
            );

            // Check existence proofs.
            for i in 1..=data_insert_count {
                // Generate proof.
                let (value, proof) = ctx
                    .get_state_proof(
                        "data",
                        S::serialize::<Vec<u8>>(&format!("key{i}").as_bytes().to_vec()),
                    )
                    .unwrap();
                // TODO(snormore): Fix this unwrap.
                // TODO(snormore): Clean up the ser/deser here, it ideally should be encapsulated in
                // the strategy/context with K/V param types.
                assert_eq!(
                    value.map(|v| S::deserialize::<String>(&v.to_vec())),
                    Some(format!("value{i}"))
                );

                // Verify proof.
                // TODO(snormore): Should this be encapsulated?
                let key = S::serialize(
                    &StateTable::new("data")
                        .key(S::serialize(&format!("key{i}").as_bytes().to_vec())),
                );
                let value = S::serialize::<Vec<u8>>(&format!("value{i}").as_bytes().to_vec());
                assert!(ics23::verify_membership::<ics23::HostFunctionsManager>(
                    &proof,
                    &ics23_spec,
                    &root_hash.as_ref().to_vec(),
                    &key,
                    value.as_slice()
                ))
            }
        });
    }

    // Insert more data.
    {
        db.run(|ctx: _| {
            let mut data_table = ctx.get_table::<String, String>("data");

            for i in 1..=data_insert_count {
                data_table.insert(format!("other{i}"), format!("value{i}"));
            }
        });

        // Check state root.
        let root_hash = reader.get_state_root().unwrap();
        assert_eq!(
            root_hash,
            "b728c39bafa70fc835797f618ae1a5dcce288c6c00033139e67583d9b1970ef4",
        );
    }

    // Remove some data.
    {
        db.run(|ctx: _| {
            let mut data_table = ctx.get_table::<String, String>("data");

            data_table.remove("key3".to_string());
            data_table.remove("other5".to_string());
            data_table.remove("other9".to_string());
        });

        // Check state root.
        let root_hash = reader.get_state_root().unwrap();
        assert_eq!(
            root_hash,
            "6b9c6c8a0bc498509afaa8508e98dc6af2c6225c71e216d786d877050d397d81",
        );
    }
}

#[allow(dead_code)]
pub fn init_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default())
        .is_test(true)
        .format_timestamp(None)
        .try_init();
}
