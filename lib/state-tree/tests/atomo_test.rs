use atomo::{AtomoBuilder, InMemoryStorage, StorageBackend};
use state_tree::{KeccakHasher, StateTreeBuilder, StateTreeWriter};

#[test]
fn test_atomo() {
    type KeyHasher = blake3::Hasher;
    type ValueHasher = KeccakHasher;

    let storage = InMemoryStorage::default();
    let builder = StateTreeBuilder::<_, KeyHasher, ValueHasher>::new(storage);
    // TODO(snormore): Return reader (along with writer) on build
    let mut db = AtomoBuilder::new(builder)
        .with_table::<u8, u8>("data")
        .with_table::<Vec<u8>, Vec<u8>>("%state_tree_nodes")
        .enable_iter("data")
        .enable_iter("%state_tree_nodes")
        .build()
        .unwrap();

    db.run(
        |ctx: &mut atomo::TableSelector<
            StateTreeWriter<InMemoryStorage, KeyHasher, ValueHasher>,
            atomo::BincodeSerde,
        >| {
            let mut data_table = ctx.get_table::<u8, u8>("data");

            data_table.insert(0, 17);
            data_table.insert(1, 18);
            data_table.insert(2, 19);
        },
    );

    {
        let storage = db.get_storage_backend_unsafe();

        let data_table_id = 0;
        let keys = storage.keys(data_table_id); // data table
        assert_eq!(keys.len(), 3);
        assert_eq!(storage.get(data_table_id, &[0]), Some(vec![17]));
        assert_eq!(storage.get(data_table_id, &[1]), Some(vec![18]));
        assert_eq!(storage.get(data_table_id, &[2]), Some(vec![19]));

        let tree_table_id = 1;
        let keys = storage.keys(tree_table_id); // tree table
        assert_eq!(keys.len(), 4);

        // This is specific to the JMT implementation.
        assert!(storage.contains(tree_table_id, &[0; 20]))
    }

    db.query().run(|ctx: _| {
        let data_table = ctx.get_table::<u8, u8>("data");
        let tree_table = ctx.get_table::<Vec<u8>, Vec<u8>>("%state_tree_nodes");

        let keys = data_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), 3);
        assert_eq!(data_table.get(0), Some(17));
        assert_eq!(data_table.get(1), Some(18));
        assert_eq!(data_table.get(2), Some(19));

        // We expect keys to be empty for the tree data when accessed through the atomo table
        // reference since keys are not populated in the atomo table reference for the tree table,
        // which does not insert through the table reference insert method, and so does not update
        // the atomo table reference keys on insert.
        let keys = tree_table.keys().collect::<Vec<_>>();
        assert_eq!(keys.len(), 0);

        // TODO(snormore): Test get_with_proof on reader
    });
}
