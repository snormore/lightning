use std::sync::{Arc, Mutex};

use anyhow::{anyhow, bail, Context};
use arrayref::array_ref;
use cid::Cid;
use deno_core::url::Url;
use deno_core::v8::{Global, IsolateHandle, Value};
use deno_core::{serde_v8, v8, JsRuntime};
use fn_sdk::connection::Connection;
use fn_sdk::header::TransportDetail;
use fn_sdk::http_util::{respond, respond_with_error, respond_with_http_response};
use lazy_static::lazy_static;
use prometheus::{self, register_int_gauge, IntGauge};
use prometheus_exporter::{self};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, error, info};

use crate::runtime::Runtime;
use crate::stream::{Origin, Request};

mod http;
mod runtime;
pub mod stream;

lazy_static! {
    static ref ACTIVE_CONNECTIONS_GAUGE: IntGauge = register_int_gauge!(
        "services_js_active_connections",
        "Gauge for number of active connections to the JS service"
    )
    .unwrap();
    static ref ACTIVE_REQUESTS_GAUGE: IntGauge = register_int_gauge!(
        "services_js_active_requests",
        "Gauge for number of active requests to the JS service"
    )
    .unwrap();
}

/// Structure for tracking active connections.
struct ActiveConnections {
    count: i64,
}

impl ActiveConnections {
    fn new() -> Self {
        ACTIVE_CONNECTIONS_GAUGE.set(0);
        Self { count: 0 }
    }

    fn inc(&mut self) {
        self.count += 1;
        ACTIVE_CONNECTIONS_GAUGE.set(self.count);
    }

    fn dec(&mut self) {
        self.count -= 1;
        ACTIVE_CONNECTIONS_GAUGE.set(self.count);
    }
}

impl Drop for ActiveConnections {
    fn drop(&mut self) {
        self.count -= 1;
        ACTIVE_CONNECTIONS_GAUGE.set(self.count);
    }
}

/// Structure for tracking active requests
struct ActiveRequests {
    count: i64,
}

impl ActiveRequests {
    fn new() -> Self {
        ACTIVE_REQUESTS_GAUGE.set(0);
        Self { count: 0 }
    }

    fn inc(&mut self) {
        self.count += 1;
        ACTIVE_REQUESTS_GAUGE.set(self.count);
    }

    fn dec(&mut self) {
        self.count -= 1;
        ACTIVE_REQUESTS_GAUGE.set(self.count);
    }
}

pub(crate) mod params {
    use std::time::Duration;

    pub const HEAP_INIT: usize = 1 << 10;
    pub const HEAP_LIMIT: usize = 50 << 20;
    pub const REQ_TIMEOUT: Duration = Duration::from_secs(15);
    pub const FETCH_BLACKLIST: &[&str] = &["localhost", "127.0.0.1", "::1"];
}

#[tokio::main(flavor = "current_thread")]
pub async fn main() {
    fn_sdk::ipc::init_from_env();

    info!("Initialized POC JS service!");

    // TODO(snormore): This is temporary for debugging purposes, and will not be merged into main
    // as-is.
    let metrics_binding = "127.0.0.1:19100".parse().unwrap();
    prometheus_exporter::start(metrics_binding).unwrap();

    let mut listener = fn_sdk::ipc::conn_bind().await;

    // Explicitly initialize the v8 platform on the main thread
    JsRuntime::init_platform(None);

    // To cancel events mid execution.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<IsolateHandle>();
    tokio::spawn(async move {
        while let Some(handle) = rx.recv().await {
            tokio::spawn(async move {
                tokio::time::sleep(params::REQ_TIMEOUT).await;
                handle.terminate_execution();
            });
        }
    });

    let active_connections = Arc::new(Mutex::new(ActiveConnections::new()));
    let active_requests = Arc::new(Mutex::new(ActiveRequests::new()));

    while let Ok(conn) = listener.accept().await {
        let tx = tx.clone();

        // spawn a new thread and tokio runtime to handle the connection
        // TODO: This is very hacky and not very scalable
        // Research using deno's JsRealms to provide the script sandboxing in a single or a
        // few shared multithreaded runtimes, or use a custom work scheduler.
        let active_connections = active_connections.clone();
        let active_requests = active_requests.clone();
        std::thread::spawn(move || {
            if let Err(e) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to create connection async runtime")
                .block_on(handle_connection(
                    tx,
                    conn,
                    active_connections,
                    active_requests,
                ))
            {
                error!("session failed: {e:?}");
            }
        });
    }
}

struct ConnectionTracker {
    tracker: Arc<Mutex<ActiveConnections>>,
}

impl ConnectionTracker {
    fn new(tracker: Arc<Mutex<ActiveConnections>>) -> Self {
        tracker.lock().unwrap().inc();
        Self { tracker }
    }
}

impl Drop for ConnectionTracker {
    fn drop(&mut self) {
        self.tracker.lock().unwrap().dec();
    }
}

async fn handle_connection(
    tx: UnboundedSender<IsolateHandle>,
    mut connection: Connection,
    active_connections: Arc<Mutex<ActiveConnections>>,
    active_requests: Arc<Mutex<ActiveRequests>>,
) -> anyhow::Result<()> {
    let _tracker = ConnectionTracker::new(active_connections);

    if connection.is_http_request() {
        let body = connection
            .read_payload()
            .await
            .context("Could not read body.")?;
        let TransportDetail::HttpRequest {
            method,
            ref url,
            ref header,
        } = connection.header.transport_detail
        else {
            unreachable!()
        };
        let request = http::request::extract(url, header, method, body.to_vec())
            .context("failed to parse request")?;
        handle_request(&mut connection, &tx, request, active_requests.clone()).await?;
    } else {
        while let Some(payload) = connection.read_payload().await {
            let request: Request = serde_json::from_slice(&payload)?;
            handle_request(&mut connection, &tx, request, active_requests.clone()).await?;
        }
    }

    Ok(())
}

struct RequestTracker {
    tracker: Arc<Mutex<ActiveRequests>>,
}

impl RequestTracker {
    fn new(tracker: Arc<Mutex<ActiveRequests>>) -> Self {
        tracker.lock().unwrap().inc();
        Self { tracker }
    }
}

impl Drop for RequestTracker {
    fn drop(&mut self) {
        self.tracker.lock().unwrap().dec();
    }
}

async fn handle_request(
    connection: &mut Connection,
    tx: &UnboundedSender<IsolateHandle>,
    request: Request,
    active_requests: Arc<Mutex<ActiveRequests>>,
) -> anyhow::Result<()> {
    let Request {
        origin,
        uri,
        path,
        param,
    } = request;

    let _tracker = RequestTracker::new(active_requests);

    // Fetch content from origin
    let hash = match origin {
        Origin::Blake3 => {
            let hash = hex::decode(uri).context("failed to decode blake3 hash")?;

            if hash.len() != 32 {
                respond_with_error(connection, b"Invalid blake3 hash length", 400).await?;
                return Err(anyhow!("invalid blake3 hash length"));
            }

            let hash = *array_ref![hash, 0, 32];

            if fn_sdk::api::fetch_blake3(hash).await {
                hash
            } else {
                respond_with_error(connection, b"Failed to fetch blake3 content", 400).await?;
                return Err(anyhow!("failed to fetch file"));
            }
        },
        Origin::Ipfs | Origin::Http => {
            let uri = match origin {
                Origin::Ipfs => Cid::try_from(uri).context("Invalid IPFS CID")?.to_bytes(),
                Origin::Http => urlencoding::decode(&uri)
                    .context("Invalid URL encoding")?
                    .to_string()
                    .into(),
                _ => unreachable!(),
            };

            match fn_sdk::api::fetch_from_origin(origin.into(), uri).await {
                Some(hash) => hash,
                None => {
                    respond_with_error(connection, b"Failed to fetch from origin", 400).await?;
                    return Err(anyhow!("failed to fetch from origin"));
                },
            }
        },
        o => {
            let err = anyhow!("unknown origin: {o:?}");
            respond_with_error(connection, err.to_string().as_bytes(), 400).await?;
            return Err(err);
        },
    };

    let mut location = Url::parse(&format!("blake3://{}", hex::encode(hash)))
        .context("failed to create base url")?;
    if let Some(path) = path {
        location = location.join(&path).context("invalid path string")?;
    }

    // Read and parse the source from the blockstore
    let source_bytes = fn_sdk::blockstore::ContentHandle::load(&hash)
        .await
        .context("failed to get handle for source from blockstore")?
        .read_to_end()
        .await
        .context("failed to read source from blockstore")?;
    let source = String::from_utf8(source_bytes).context("failed to parse source as utf8")?;

    // Create runtime and execute the source
    let mut runtime = match Runtime::new(location.clone()) {
        Ok(runtime) => runtime,
        Err(e) => {
            respond_with_error(connection, e.to_string().as_bytes(), 400).await?;
            return Err(e).context("failed to initialize runtime");
        },
    };

    tx.send(runtime.deno.v8_isolate().thread_safe_handle())
        .context("Failed to send the IsolateHandle to main thread.")?;

    let res = match runtime.exec(location, source, param).await {
        Ok(Some(res)) => res,
        Ok(None) => {
            respond_with_error(connection, b"no response available", 400).await?;
            bail!("no response available");
        },
        Err(e) => {
            respond_with_error(connection, e.to_string().as_bytes(), 400).await?;
            return Err(e).context("failed to run javascript");
        },
    };

    // Resolve async if applicable
    // TODO: figure out why `deno.resolve` doesn't drive async functions
    #[allow(deprecated)]
    let res = match tokio::time::timeout(params::REQ_TIMEOUT, runtime.deno.resolve_value(res)).await
    {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => {
            respond_with_error(connection, e.to_string().as_bytes(), 400).await?;
            return Err(e).context("failed to resolve output");
        },
        Err(e) => {
            respond_with_error(connection, b"Request timeout", 400).await?;
            return Err(e).context("execution timeout");
        },
    };

    parse_and_respond(connection, &mut runtime, res).await?;

    let feed = runtime.end();
    debug!("{feed:?}");

    Ok(())
}

async fn parse_and_respond(
    connection: &mut Connection,
    runtime: &mut Runtime,
    res: Global<Value>,
) -> anyhow::Result<()> {
    // Handle the return data
    let scope = &mut runtime.deno.handle_scope();
    let local = v8::Local::new(scope, res);

    if local.is_uint8_array() || local.is_array_buffer() {
        // If the return type is a U8 array, send the raw data directly to the client
        let bytes = match deno_core::_ops::to_v8_slice_any(local) {
            Ok(slice) => slice.to_vec(),
            Err(e) => return Err(anyhow!("failed to parse bytes: {e}")),
        };
        respond(connection, &bytes).await?;
    } else if local.is_string() {
        // Likewise for string types
        let string = serde_v8::from_v8::<String>(scope, local)
            .context("failed to deserialize response string")?;

        respond(connection, string.as_bytes()).await?;
    } else {
        // Parse the response into a generic json value
        let value = serde_v8::from_v8::<serde_json::Value>(scope, local)
            .context("failed to deserialize response")?
            .clone();

        // Attempt to parse and use the value as an http response override object
        if connection.is_http_request() {
            if let Ok(http_response) = http::response::parse(&value) {
                respond_with_http_response(connection, http_response).await?;
                return Ok(());
            }
        }

        // Otherwise, send the data as a json string
        let res = serde_json::to_string(&value).context("failed to encode json response")?;
        respond(connection, res.as_bytes()).await?;
    }

    Ok(())
}
