use atomo::{DefaultSerdeBackend, InMemoryStorage, SerdeBackend, StorageBackend};
use atomo_merklized::{KeccakHasher, MerklizedAtomoBuilder, TableKey};

#[test]
fn test_atomo() {
    let storage = InMemoryStorage::default();
    let mut db = MerklizedAtomoBuilder::<
        InMemoryStorage,
        DefaultSerdeBackend,
        blake3::Hasher,
        KeccakHasher,
    >::new(storage)
    .with_table::<String, String>("data")
    .enable_iter("data")
    .with_table::<u8, u8>("other")
    .build()
    .unwrap();
    let reader = db.query();

    let data_insert_count = 10;

    // Insert initial data.
    db.run(|ctx: _| {
        let mut data_table = ctx.get_table::<String, String>("data");

        for i in 1..=data_insert_count {
            data_table.insert(format!("key{i}"), format!("value{i}"));
        }
    });

    // Verify data via storage directly.
    {
        let storage = db.get_storage_backend_unsafe();

        let data_table_id = 0;
        let keys = storage.keys(data_table_id);
        assert_eq!(keys.len(), data_insert_count);

        // TODO(snormore): Can we get this table index via the table ref instead of indirectly
        // inferring it?
        let tree_table_id = 2;
        let keys = storage.keys(tree_table_id);
        assert_eq!(keys.len(), 12);
    }

    // Check state root.
    let root_hash = reader.get_state_root().unwrap();
    assert_eq!(
        root_hash,
        "0xf3e46a84409c4b1cdf2cc51d60137acb3afccdccc6e2822b9c5d641c5ef95157"
    );

    // Verify data via state tree reader.
    reader.run(|ctx| {
        let data_table = ctx.get_table::<String, String>("data");

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
            "0xf3e46a84409c4b1cdf2cc51d60137acb3afccdccc6e2822b9c5d641c5ef95157"
        );

        // Check tree table key count.
        let tree_table = ctx.state_tree_table();
        let keys = tree_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), 12);

        // Check existence proofs.
        for i in 1..=data_insert_count {
            let (value, proof) = data_table.get_with_proof(format!("key{i}"));
            assert_eq!(value, Some(format!("value{i}")));

            // TODO(snormore): Make our own proof type and avoid constructing a keyhash out here.
            let key = TableKey {
                table: "data".to_string(),
                key: DefaultSerdeBackend::serialize(&format!("key{i}").as_bytes().to_vec()),
            };
            let key_hash = key.hash::<DefaultSerdeBackend, blake3::Hasher>();
            let value: Vec<u8> =
                DefaultSerdeBackend::serialize(&format!("value{i}").as_bytes().to_vec());
            proof
                .verify_existence(jmt::RootHash(root_hash.into()), key_hash, value)
                .unwrap();
        }
    });
}
