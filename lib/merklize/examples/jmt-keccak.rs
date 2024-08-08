use atomo::{
    AtomoBuilder,
    DefaultSerdeBackend,
    InMemoryStorage,
    SerdeBackend,
    StorageBackendConstructor,
};
use merklize::hashers::keccak::KeccakHasher;
use merklize::providers::jmt::JmtMerklizeProvider;
use merklize::{MerklizeProvider, MerklizedAtomo, StateProof};

pub fn main() {
    let builder = InMemoryStorage::default();

    run::<_, JmtMerklizeProvider<_, DefaultSerdeBackend, KeccakHasher>>(builder);
}

fn run<B: StorageBackendConstructor, M: MerklizeProvider<Storage = B::Storage>>(builder: B) {
    let mut db = MerklizedAtomo::<_, _, _, M>::new(
        M::with_tables(AtomoBuilder::new(builder).with_table::<String, String>("data"))
            .build()
            .unwrap(),
    );
    let query = db.query();

    // Open writer context and insert some data.
    db.run(|ctx| {
        let mut table = ctx.get_table::<String, String>("data");

        // Insert data.
        table.insert("key".to_string(), "value".to_string());
    });

    // Open reader context, read the data, get the state root hash, and get a proof of existence.
    query.run(|ctx| {
        let table = ctx.get_table::<String, String>("data");

        // Read the data.
        let value = table.get("key".to_string()).unwrap();
        println!("value: {:?}", value);

        // Get the merklize context.
        let ctx = M::context(ctx);

        // Get the state root hash.
        let state_root = ctx.get_state_root().unwrap();
        println!("state root: {:?}", state_root);

        // Get a proof of existence for some value in the state.
        let proof = ctx
            .get_state_proof("data", M::Serde::serialize(&"key"))
            .unwrap();
        println!("proof: {:?}", proof);

        // Verify the proof.
        proof
            .verify_membership::<String, String, M>(
                "data",
                "key".to_string(),
                "value".to_string(),
                state_root,
            )
            .unwrap();
    });
}
