use atomo::{DefaultSerdeBackend, InMemoryStorage, StorageBackend};
use atomo_merklized::{KeccakHasher, MerklizedAtomoBuilder, MerklizedLayout};
use atomo_merklized_jmt::JmtMerklizedStrategy;

#[test]
fn test_atomo() {
    #[derive(Clone)]
    pub struct TestLayout;

    impl MerklizedLayout for TestLayout {
        type SerdeBackend = DefaultSerdeBackend;
        type Strategy = JmtMerklizedStrategy<Self>;
        type KeyHasher = blake3::Hasher;
        type ValueHasher = KeccakHasher;
    }

    let storage = InMemoryStorage::default();
    let mut db = MerklizedAtomoBuilder::<InMemoryStorage, TestLayout>::new(storage)
        // TODO(snormore): Should the following happen internally to the StateTable implementation?
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
            // TODO(snormore): Encapsulate this in a state table trait method `get_table_reference`.
            let mut data_table = ctx.get_table::<String, String>("data");

            for i in 1..=data_insert_count {
                data_table.insert(format!("key{i}"), format!("value{i}"));
            }
        });

        // Check state root.
        let root_hash = reader.get_state_root().unwrap();
        assert_eq!(
            root_hash,
            "f3e46a84409c4b1cdf2cc51d60137acb3afccdccc6e2822b9c5d641c5ef95157"
        );

        // Check data in storage.
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

        // Check data via reader.
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
                "f3e46a84409c4b1cdf2cc51d60137acb3afccdccc6e2822b9c5d641c5ef95157"
            );

            // Check tree table key count.
            let tree_table = ctx.state_tree_table();
            let keys = tree_table.keys().collect::<Vec<_>>();
            assert_eq!(keys.len(), 12);

            // Check existence proofs.
            for i in 1..=data_insert_count {
                let (value, _proof) = data_table.get_with_proof(format!("key{i}"));
                assert_eq!(value, Some(format!("value{i}")));

                // TODO(snormore): Make our own proof type and avoid constructing a keyhash out
                // here. let key_hash = StateTable::new("data".to_string())
                //     .key(DefaultSerdeBackend::serialize(&format!("key{i}").as_bytes().to_vec()).
                // into())     .hash::<DefaultSerdeBackend, blake3::Hasher>();
                // let value: Vec<u8> =
                //     DefaultSerdeBackend::serialize(&format!("value{i}").as_bytes().to_vec());
                // proof
                //     .verify_existence(
                //         jmt::RootHash(root_hash.into()),
                //         jmt::KeyHash(key_hash.into()),
                //         value,
                //     )
                //     .unwrap();
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
            "24d94d1ec858e9d3cd043683777ce9f345fe9c121fdee0727c1d9bfa7dd17e99",
        );

        // Check data in storage.
        {
            let storage = db.get_storage_backend_unsafe();

            let data_table_id = 0;
            let keys = storage.keys(data_table_id);
            assert_eq!(keys.len(), 2 * data_insert_count);

            // TODO(snormore): Can we get this table index via the table ref instead of indirectly
            // inferring it?
            let tree_table_id = 2;
            let keys = storage.keys(tree_table_id);
            assert_eq!(keys.len(), 22);
        }
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
            "efe212a8ae9804fd99841fd1c7ead73a8e1d8856174c5a6de1c6bee8b6c74a64",
        );

        // Check data in storage.
        {
            let storage = db.get_storage_backend_unsafe();

            let data_table_id = 0;
            let keys = storage.keys(data_table_id);
            assert_eq!(keys.len(), 2 * data_insert_count - 3);

            // TODO(snormore): Can we get this table index via the table ref instead of indirectly
            // inferring it?
            let tree_table_id = 2;
            let keys = storage.keys(tree_table_id);
            assert_eq!(keys.len(), 20);
        }
    }
}
