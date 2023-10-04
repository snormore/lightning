use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use affair::{Socket, Task};
use anyhow::anyhow;
use async_trait::async_trait;
use lightning_interfaces::infu_collection::Collection;
use lightning_interfaces::types::{Blake3Hash, FetcherRequest, FetcherResponse, ImmutablePointer};
use lightning_interfaces::{
    BlockStoreInterface,
    ConfigConsumer,
    FetcherInterface,
    FetcherSocket,
    OriginProviderInterface,
    OriginProviderSocket,
    PoolInterface,
    ResolverInterface,
    ServiceScope,
    WithStartAndShutdown,
};
use log::error;
use tokio::sync::{mpsc, oneshot};

use crate::blockstore_server::{BlockstoreServer, ServerRequest};
use crate::config::Config;
use crate::origin::{OriginFetcher, OriginRequest};

pub(crate) type Uri = Vec<u8>;

pub struct Fetcher<C: Collection> {
    inner: Arc<FetcherInner<C>>,
    is_running: Arc<AtomicBool>,
    socket: FetcherSocket,
    shutdown_tx: mpsc::Sender<()>,
}

#[async_trait]
impl<C: Collection> FetcherInterface<C> for Fetcher<C> {
    /// Initialize the fetcher.
    fn init(
        config: Self::Config,
        blockstore: C::BlockStoreInterface,
        resolver: C::ResolverInterface,
        origin: &C::OriginProviderInterface,
        pool: &C::PoolInterface,
    ) -> anyhow::Result<Self> {
        let (pool_requester, pool_responder) = pool.open_req_res(ServiceScope::BlockstoreServer);
        let (request_tx, request_rx) = mpsc::channel(1024);
        let blockstore_server = BlockstoreServer::<C>::init(
            blockstore.clone(),
            request_rx,
            config.max_conc_req,
            config.max_conc_res,
            pool_requester,
            pool_responder,
        )?;

        let (socket, socket_rx) = Socket::raw_bounded(2048);
        let (shutdown_tx, shutdown_rx) = mpsc::channel(10);
        let inner = FetcherInner::<C> {
            config,
            socket_rx: Arc::new(Mutex::new(Some(socket_rx))),
            origin_socket: origin.get_socket(),
            blockstore,
            blockstore_server,
            server_request_tx: request_tx,
            resolver,
            shutdown_rx: Arc::new(Mutex::new(Some(shutdown_rx))),
        };

        Ok(Self {
            inner: Arc::new(inner),
            is_running: Arc::new(AtomicBool::new(false)),
            socket,
            shutdown_tx,
        })
    }

    fn get_socket(&self) -> FetcherSocket {
        self.socket.clone()
    }
}

#[async_trait]
impl<C: Collection> WithStartAndShutdown for Fetcher<C> {
    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    async fn start(&self) {
        if !self.is_running() {
            let inner = self.inner.clone();
            let is_running = self.is_running.clone();
            tokio::spawn(async move {
                inner.start().await;
                is_running.store(false, Ordering::Relaxed);
            });
            self.is_running.store(true, Ordering::Relaxed);
        } else {
            error!("Cannot start fetcher because it is already running");
        }
    }

    async fn shutdown(&self) {
        self.shutdown_tx
            .send(())
            .await
            .expect("Failed to send shutdown signal");
    }
}

struct FetcherInner<C: Collection> {
    config: Config,
    #[allow(clippy::type_complexity)]
    socket_rx: Arc<Mutex<Option<mpsc::Receiver<Task<FetcherRequest, FetcherResponse>>>>>,
    origin_socket: OriginProviderSocket,
    blockstore: C::BlockStoreInterface,
    blockstore_server: BlockstoreServer<C>,
    server_request_tx: mpsc::Sender<ServerRequest>,
    resolver: C::ResolverInterface,
    shutdown_rx: Arc<Mutex<Option<mpsc::Receiver<()>>>>,
}

impl<C: Collection> FetcherInner<C> {
    async fn start(&self) {
        let mut socket_rx = self.socket_rx.lock().unwrap().take().unwrap();

        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (tx, rx) = mpsc::channel(128);
        let origin_fetcher = OriginFetcher::<C>::new(
            self.config.max_conc_origin_req,
            self.origin_socket.clone(),
            rx,
            self.resolver.clone(),
            shutdown_rx,
        );
        tokio::spawn(async move {
            origin_fetcher.start().await;
        });
        self.blockstore_server.start().await;
        let mut shutdown_rx = self.shutdown_rx.lock().unwrap().take().unwrap();

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    shutdown_tx.send(()).expect("Failed to send shutdown signal to origin fetcher");
                    break;
                }
                task = socket_rx.recv() => {
                    let task = task.expect("Failed to receive fetcher request.");
                    match task.request.clone() {
                        FetcherRequest::Put { pointer } => {
                            let tx = tx.clone();
                            let resolver = self.resolver.clone();
                            tokio::spawn(async move {
                                FetcherInner::<C>::put(pointer, tx, resolver, task).await;
                            });
                        }
                        FetcherRequest::Fetch { hash } => {
                            let tx = tx.clone();
                            let resolver = self.resolver.clone();
                            let blockstore = self.blockstore.clone();
                            let server_request_tx = self.server_request_tx.clone();
                            tokio::spawn(async move {
                                FetcherInner::<C>::fetch(
                                    hash,
                                    tx,
                                    resolver,
                                    blockstore,
                                    server_request_tx,
                                    task
                                ).await;
                            });
                        }
                    }
                }
            }
        }
        *self.socket_rx.lock().unwrap() = Some(socket_rx);
        *self.shutdown_rx.lock().unwrap() = Some(shutdown_rx);
        self.blockstore_server.shutdown().await;
    }

    /// Fetches the data from the corresponding origin, puts it in the blockstore, and stores the
    /// mapping using the resolver. If the mapping already exists, the data will not be fetched
    /// from origin again.
    async fn put(
        pointer: ImmutablePointer,
        tx: mpsc::Sender<OriginRequest>,
        resolver: C::ResolverInterface,
        task: Task<FetcherRequest, FetcherResponse>,
    ) {
        if let Some(resolved_pointer) = resolver.get_blake3_hash(pointer.clone()).await {
            task.respond(FetcherResponse::Put(Ok(resolved_pointer.hash)));
        } else {
            let (response_tx, response_rx) = oneshot::channel();
            tx.send(OriginRequest {
                pointer,
                response: response_tx,
            })
            .await
            .expect("Failed to send origin request");

            let mut hash_rx = response_rx.await.expect("Failed to receive response");

            match hash_rx.recv().await.expect("Failed to receive response") {
                Ok(hash) => task.respond(FetcherResponse::Put(Ok(hash))),
                Err(e) => task.respond(FetcherResponse::Put(Err(anyhow!("{e:?}")))),
            }
        }
    }

    /// Fetches the data from the blockstore. If the data does not exist in the blockstore, it will
    /// be fetched from the origin.
    async fn fetch(
        hash: Blake3Hash,
        tx: mpsc::Sender<OriginRequest>,
        resolver: C::ResolverInterface,
        blockstore: C::BlockStoreInterface,
        server_request_tx: mpsc::Sender<ServerRequest>,
        task: Task<FetcherRequest, FetcherResponse>,
    ) {
        if blockstore.get_tree(&hash).await.is_some() {
            task.respond(FetcherResponse::Fetch(Ok(())));
            return;
        } else if let Some(pointers) = resolver.get_origins(hash) {
            for res_pointer in pointers {
                if let Some(res) = resolver.get_blake3_hash(res_pointer.pointer.clone()).await {
                    if res.hash == hash && blockstore.get_tree(&hash).await.is_some() {
                        task.respond(FetcherResponse::Fetch(Ok(())));
                        return;
                    }
                    // Try to get the content from the peer that advertized the record.
                    let (tx, rx) = oneshot::channel();
                    server_request_tx
                        .send(ServerRequest {
                            hash,
                            peer: res.originator,
                            response: tx,
                        })
                        .await
                        .expect("Failed to send request to blockstore server");
                    let mut res = rx.await.expect("Failed to get receiver");
                    if let Ok(()) = res
                        .recv()
                        .await
                        .expect("Failed to receive response from blockstore server")
                    {
                        task.respond(FetcherResponse::Fetch(Ok(())));
                        return;
                    }
                }
                // If the content is neither in the blockstore and we cannot get it from our peers,
                // we go to the origin.
                let (response_tx, response_rx) = oneshot::channel();
                tx.send(OriginRequest {
                    pointer: res_pointer.pointer,
                    response: response_tx,
                })
                .await
                .expect("Failed to send origin request");

                let mut hash_rx = response_rx.await.expect("Failed to receive response");
                if let Ok(hash) = hash_rx.recv().await.expect("Failed to receive response") {
                    task.respond(FetcherResponse::Put(Ok(hash)));
                    return;
                }
            }
        }
        task.respond(FetcherResponse::Fetch(Err(anyhow!(
            "Failed to resolve hash"
        ))));
    }

    //async fn fetch_from_origin(
    //    hash: Blake3Hash,
    //    origin_tx: mpsc::Sender<OriginRequest>,
    //    resolver: C::ResolverInterface,
    //    blockstore: C::BlockStoreInterface,
    //) -> Result<()> {
    //    if let Some(pointers) = resolver.get_origins(hash) {
    //        for res_pointer in pointers {
    //            if let Some(res) = resolver.get_blake3_hash(res_pointer.pointer.clone()).await {
    //                if res.hash == hash && blockstore.get_tree(&hash).await.is_some() {
    //                    return Ok(());
    //                }
    //            } else {
    //                let (response_tx, response_rx) = oneshot::channel();
    //                origin_tx
    //                    .send(OriginRequest {
    //                        pointer: res_pointer.pointer,
    //                        response: response_tx,
    //                    })
    //                    .await
    //                    .expect("Failed to send origin request");

    //                let mut hash_rx = response_rx.await.expect("Failed to receive response");
    //                if let Ok(_hash) = hash_rx.recv().await.expect("Failed to receive response") {
    //                    return Ok(());
    //                }
    //            }
    //        }
    //    }
    //    Err(anyhow!("Failed to retrieve content"))
    //}
}

impl<C: Collection> ConfigConsumer for Fetcher<C> {
    const KEY: &'static str = "fetcher";

    type Config = Config;
}
