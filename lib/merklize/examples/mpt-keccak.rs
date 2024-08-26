use atomo::{AtomoBuilder, DefaultSerdeBackend, InMemoryStorage, SerdeBackend};
use merklize::hashers::keccak::KeccakHasher;
use merklize::providers::mpt::MptStateTree;
use merklize::{StateProof, StateTree, StateTreeReader};

pub fn main() {
    let builder = InMemoryStorage::default();

    run::<MptStateTree<_, DefaultSerdeBackend, KeccakHasher>>(builder);
}

fn run<T: StateTree>(builder: T::StorageBuilder) {
    let mut db =
        T::register_tables(AtomoBuilder::new(builder).with_table::<String, String>("data"))
            .build()
            .unwrap();
    let query = db.query();
    let tree = T::new();
    let tree_reader = tree.reader(query.clone());

    // Open writer context and insert some data.
    db.run(|ctx| {
        let mut table = ctx.get_table::<String, String>("data");

        // Insert data.
        table.insert("key".to_string(), "value".to_string());

        // Update state tree.
        tree.update_state_tree_from_context(ctx).unwrap();
    });

    // Open reader context, read the data, get the state root hash, and get a proof of existence.
    query.run(|ctx| {
        let table = ctx.get_table::<String, String>("data");

        // Read the data.
        let value = table.get("key".to_string()).unwrap();
        println!("value: {:?}", value);

        // Get the state root hash.
        let state_root = tree_reader.get_state_root(ctx).unwrap();
        println!("state root: {:?}", state_root);

        // Get a proof of existence for some value in the state.
        let proof = tree_reader
            .get_state_proof(ctx, "data", <T::Serde as SerdeBackend>::serialize(&"key"))
            .unwrap();
        println!("proof: {:?}", proof);

        // Verify the proof.
        proof
            .verify_membership::<String, String, T>(
                "data",
                "key".to_string(),
                "value".to_string(),
                state_root,
            )
            .unwrap();
    });
}
