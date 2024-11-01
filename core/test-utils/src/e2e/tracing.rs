use tracing_subscriber::prelude::*;
use tracing_subscriber::util::TryInitError;
use tracing_subscriber::EnvFilter;

pub struct TracingOptions {
    pub with_console: bool,
}

#[allow(unused)]
pub fn try_init_tracing(options: Option<TracingOptions>) -> Result<(), TryInitError> {
    let env_filter = EnvFilter::builder()
        .from_env()
        .unwrap()
        .add_directive("anemo=warn".parse().unwrap())
        .add_directive("rustls=warn".parse().unwrap())
        .add_directive("h2=warn".parse().unwrap())
        .add_directive("tokio=warn".parse().unwrap())
        .add_directive("runtime=warn".parse().unwrap());
    let registry = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_thread_names(true)
            .with_filter(env_filter),
    );
    if let Some(options) = options {
        if options.with_console {
            let console_layer = console_subscriber::Builder::default()
                .with_default_env()
                .server_addr(([0, 0, 0, 0], 6669))
                .spawn();
            registry.with(console_layer).try_init()?;
            return Ok(());
        }
    }
    registry.try_init()
}

#[allow(unused)]
pub fn init_tracing(options: Option<TracingOptions>) {
    try_init_tracing(options).expect("failed to initialize tracing");
}

#[allow(unused)]
pub fn try_init_tracing_with_tokio_console() -> Result<(), TryInitError> {
    try_init_tracing(Some(TracingOptions { with_console: true }))
}

#[allow(unused)]
pub fn init_tracing_with_tokio_console() {
    try_init_tracing_with_tokio_console().expect("failed to initialize tracing with tokio console");
}
