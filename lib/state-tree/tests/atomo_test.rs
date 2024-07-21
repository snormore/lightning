use atomo::{DefaultSerdeBackend, InMemoryStorage, StorageBackend};
use state_tree::{KeccakHasher, SerializedNodeKey, SerializedNodeValue, StateTreeBuilder};

#[test]
fn test_atomo() {
    let storage = InMemoryStorage::default();
    let mut db = StateTreeBuilder::<
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
        assert_eq!(keys.len(), 11);
    }

    // Verify data via state tree reader.
    db.query().run(|ctx: _| {
        let data_table = ctx.get_table::<String, String>("data");

        let keys = data_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), data_insert_count);

        for i in 1..=data_insert_count {
            assert_eq!(data_table.get(format!("key{i}")), Some(format!("value{i}")));
        }

        let root_hash = db.get_root_hash(ctx).unwrap();
        assert_eq!(
            hex::encode(root_hash),
            "f99c316badabfe6c5a22f7697d2465dd81dfade2ca46464fa3f1000c850ff66f"
        );

        let tree_table =
            ctx.get_table::<SerializedNodeKey, SerializedNodeValue>("%state_tree_nodes");
        let keys = tree_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), 11);

        // TODO(snormore): Test get_with_proof on reader
    });
}
