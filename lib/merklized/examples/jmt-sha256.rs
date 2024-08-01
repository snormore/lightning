use atomo::{DefaultSerdeBackend, InMemoryStorage, StorageBackendConstructor};
use merklized::hashers::sha2::Sha256Hasher;
use merklized::strategies::jmt::JmtMerklizedStrategy;
use merklized::{MerklizedAtomoBuilder, MerklizedStrategy};

pub fn main() {
    let builder = InMemoryStorage::default();

    run::<_, JmtMerklizedStrategy<_, DefaultSerdeBackend, Sha256Hasher>>(builder);
}

fn run<B: StorageBackendConstructor, M: MerklizedStrategy<Storage = B::Storage>>(builder: B) {
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

        // Get the merklized context.
        let ctx = M::context(ctx);

        // Get the state root hash.
        let root_hash = ctx.get_state_root().unwrap();
        println!("state root: {:?}", root_hash);

        // Get a proof of existence for some value in the state.
        let (value, proof) = ctx
            .get_state_proof("data", "key".as_bytes().to_vec())
            .unwrap();
        println!("value: {:?}", value);
        println!("proof: {:?}", proof);
    });
}
