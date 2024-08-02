use anyhow::Result;
use atomo::{DefaultSerdeBackend, InMemoryStorage, SerdeBackend, StorageBackendConstructor};
use merklized::hashers::keccak::KeccakHasher;
use merklized::strategies::jmt::JmtMerklizedStrategy;
use merklized::{MerklizedAtomoBuilder, MerklizedStrategy};
use opentelemetry::trace::{TraceError, TracerProvider as _};
use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing::trace_span;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

/// An example of using the `[merklized]` crate with tracing.
#[tokio::main]
async fn main() -> Result<()> {
    let service_name = "merklized-tracing-example";
    let provider = init_tracer_provider(service_name.to_string())?;
    let tracer = provider.tracer(service_name);

    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry);

    tracing::subscriber::with_default(subscriber, || {
        let span = trace_span!("main");
        let _enter = span.enter();

        let builder = InMemoryStorage::default();
        run::<_, JmtMerklizedStrategy<_, DefaultSerdeBackend, KeccakHasher>>(builder, 100);
    });

    Ok(())
}

fn run<B: StorageBackendConstructor, M: MerklizedStrategy<Storage = B::Storage>>(
    builder: B,
    data_count: usize,
) {
    let mut db = MerklizedAtomoBuilder::<B, M::Serde, M>::new(builder)
        .with_table::<String, String>("data")
        .build()
        .unwrap();

    // Open writer context and insert some data.
    db.run(|ctx| {
        let mut table = ctx.get_table::<String, String>("data");

        // Insert some initial data.
        for i in 1..=data_count {
            table.insert(format!("initial-key{i}"), format!("initial-value{i}"));
        }
    });

    // Open writer context and insert some data.
    db.run(|ctx| {
        let mut table = ctx.get_table::<String, String>("data");

        // Insert data.
        for i in 1..=data_count {
            table.insert(format!("key{i}"), format!("value{i}"));
        }
    });

    // Open reader context, read the data, get the state root hash, and get a proof of existence.
    db.query().run(|ctx| {
        let table = ctx.get_table::<String, String>("data");

        // Read the data.
        let value = table.get("key1".to_string()).unwrap();
        println!("value(key1): {:?}", value);

        // Get the merklized context.
        let ctx = M::context(ctx);

        // Get the state root hash.
        let root_hash = ctx.get_state_root().unwrap();
        println!("state root: {:?}", root_hash);

        // Get a proof of existence for some value in the state.
        let (value, proof) = ctx
            .get_state_proof("data", M::Serde::serialize(&"key1"))
            .unwrap();
        println!("value: {:?}", value);
        println!("proof: {:?}", proof);
    });
}

fn init_tracer_provider(
    service_name: String,
) -> Result<opentelemetry_sdk::trace::TracerProvider, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(
            sdktrace::Config::default().with_resource(Resource::new(vec![KeyValue::new(
                SERVICE_NAME,
                service_name,
            )])),
        )
        .install_batch(runtime::Tokio)
}
