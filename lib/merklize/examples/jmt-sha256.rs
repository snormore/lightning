use atomo::{DefaultSerdeBackend, InMemoryStorage, SerdeBackend, StorageBackendConstructor};
use merklize::hashers::sha2::Sha256Hasher;
use merklize::providers::jmt::JmtMerklizeProvider;
use merklize::{MerklizeProvider, MerklizedAtomoBuilder};

pub fn main() {
    let builder = InMemoryStorage::default();

    run::<_, JmtMerklizeProvider<_, DefaultSerdeBackend, Sha256Hasher>>(builder);
}

fn run<B: StorageBackendConstructor, M: MerklizeProvider<Storage = B::Storage>>(builder: B) {
    let mut db = MerklizedAtomoBuilder::<B, M::Serde, M>::new(builder)
        .with_table::<String, String>("data")
        .build()
        .unwrap();

    // Open writer context and insert some data.
    db.run(|ctx| {
        let mut table = ctx.get_table::<String, String>("data");

        // Insert data.
        table.insert("key".to_string(), "value".to_string());
    });

    // Open reader context, read the data, get the state root hash, and get a proof of existence.
    db.query().run(|ctx| {
        let table = ctx.get_table::<String, String>("data");

        // Read the data.
        let value = table.get("key".to_string()).unwrap();
        println!("value: {:?}", value);

        // Get the merklize context.
        let ctx = M::context(ctx);

        // Get the state root hash.
        let root_hash = ctx.get_state_root().unwrap();
        println!("state root: {:?}", root_hash);

        // Get a proof of existence for some value in the state.
        let (value, _proof) = ctx
            .get_state_proof("data", M::Serde::serialize(&"key"))
            .unwrap();
        println!("value: {:?}", value);
        // println!("proof: {:?}", proof);
    });
}
