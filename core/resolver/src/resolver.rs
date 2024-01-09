use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use fleek_crypto::{NodeSecretKey, PublicKey, SecretKey};
use lightning_interfaces::infu_collection::{c, Collection};
use lightning_interfaces::schema::broadcast::ResolvedImmutablePointerRecord;
use lightning_interfaces::types::{Blake3Hash, ImmutablePointer, NodeIndex};
use lightning_interfaces::{
    ApplicationInterface,
    BroadcastInterface,
    ConfigConsumer,
    PubSub,
    ResolverInterface,
    SignerInterface,
    SyncQueryRunnerInterface,
    ToDigest,
    WithStartAndShutdown,
};
use rocksdb::{Options, DB};
use tokio::sync::{Notify, OnceCell};
use tracing::{error, warn};

use crate::config::Config;
use crate::origin_finder::OriginFinder;

const B3_TO_URI: &str = "b3_to_uri";
const URI_TO_B3: &str = "uri_to_b3";

#[derive(Clone)]
pub struct Resolver<C: Collection> {
    inner: Arc<ResolverInner<C>>,
    is_running: Arc<AtomicBool>,
    shutdown_notify: Arc<Notify>,
}

impl<C: Collection> ConfigConsumer for Resolver<C> {
    const KEY: &'static str = "resolver";

    type Config = Config;
}

impl<C: Collection> WithStartAndShutdown for Resolver<C> {
    /// Returns true if this system is running or not.
    fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Start the system, should not do anything if the system is already
    /// started.
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
            error!("Can not start resolver because it already running");
        }
    }

    /// Send the shutdown signal to the system.
    async fn shutdown(&self) {
        self.shutdown_notify.notify_one();
    }
}

impl<C: Collection> ResolverInterface<C> for Resolver<C> {
    type OriginFinder = OriginFinder;

    /// Initialize and return the resolver service.
    fn init(
        config: Self::Config,
        signer: &c!(C::SignerInterface),
        pubsub: c!(C::BroadcastInterface::PubSub<ResolvedImmutablePointerRecord>),
        query_runner: c!(C::ApplicationInterface::SyncExecutor),
    ) -> anyhow::Result<Self> {
        let (_, node_sk) = signer.get_sk();

        let mut db_options = Options::default();
        db_options.create_if_missing(true);
        db_options.create_missing_column_families(true);

        let cf = vec![B3_TO_URI, URI_TO_B3];
        // Todo(Dalton): Configure rocksdb options
        let db = Arc::new(
            DB::open_cf(&db_options, config.store_path, cf)
                .expect("Was not able to create Resolver DB"),
        );

        let shutdown_notify = Arc::new(Notify::new());
        let inner = ResolverInner {
            pubsub,
            node_sk,
            node_index: OnceCell::new(),
            db,
            shutdown_notify: shutdown_notify.clone(),
            query_runner,
        };

        Ok(Self {
            inner: Arc::new(inner),
            is_running: Arc::new(AtomicBool::new(false)),
            shutdown_notify,
        })
    }

    /// Publish new records into the resolver global hash table about us witnessing
    /// the given blake3 hash from resolving the following pointers.
    async fn publish(&self, hash: Blake3Hash, pointers: &[ImmutablePointer]) {
        self.inner.publish(hash, pointers).await;
    }

    /// Tries to find the blake3 hash of an immutable pointer by only relying on locally cached
    /// records and without performing any contact with other nodes.
    ///
    /// This can return [`None`] if no local record is found.
    async fn get_blake3_hash(&self, pointer: ImmutablePointer) -> Option<Blake3Hash> {
        self.inner.get_blake3_hash(pointer).await
    }

    /// Returns an origin finder that can yield origins for the provided blake3 hash.
    fn get_origin_finder(&self, _hash: Blake3Hash) -> Self::OriginFinder {
        todo!()
    }

    fn get_origins(&self, hash: Blake3Hash) -> Option<Vec<ResolvedImmutablePointerRecord>> {
        self.inner.get_origins(hash)
    }
}

struct ResolverInner<C: Collection> {
    pubsub: c!(C::BroadcastInterface::PubSub<ResolvedImmutablePointerRecord>),
    node_sk: NodeSecretKey,
    node_index: OnceCell<NodeIndex>,
    db: Arc<DB>,
    shutdown_notify: Arc<Notify>,
    query_runner: c!(C::ApplicationInterface::SyncExecutor),
}

impl<C: Collection> ResolverInner<C> {
    async fn start(&self) {
        let mut pubsub = self.pubsub.clone();
        let db = self.db.clone();
        let shutdown_notify = self.shutdown_notify.clone();

        loop {
            tokio::select! {
                _ = shutdown_notify.notified() => break,
                Some(record) = pubsub.recv() => {
                    match self.query_runner.index_to_pubkey(&record.originator) {
                        Some(peer_public_key) => {
                            let digest = record.to_digest();
                            peer_public_key.verify(&record.signature, &digest);
                            if peer_public_key.verify(&record.signature, &digest) {
                                ResolverInner::<C>::store_mapping(record, &db);
                            } else {
                                warn!("Received record with invalid signature")
                            }
                        },
                        None => warn!("Received record from unknown node index"),
                    }
                }
            }
        }
    }

    /// Publish new records into the resolver global hash table about us witnessing
    /// the given blake3 hash from resolving the following pointers.
    async fn publish(&self, hash: Blake3Hash, pointers: &[ImmutablePointer]) {
        if !pointers.is_empty() {
            let node_index = match self.node_index.get() {
                Some(node_index) => *node_index,
                None => {
                    let node_index = self
                        .query_runner
                        .pubkey_to_index(&self.node_sk.to_pk())
                        .expect("Called `publish` without being on the application state.");
                    self.node_index
                        .set(node_index)
                        .expect("Failed to set once cell");
                    node_index
                },
            };
            let mut resolved_pointer = ResolvedImmutablePointerRecord {
                pointer: pointers[0].clone(),
                hash,
                originator: node_index,
                signature: [0; 64].into(),
            };
            let digest = resolved_pointer.to_digest();
            resolved_pointer.signature = self.node_sk.sign(&digest);
            ResolverInner::<C>::store_mapping(resolved_pointer.clone(), &self.db);

            for (index, pointer) in pointers.iter().enumerate() {
                if index > 0 {
                    resolved_pointer.pointer = pointer.clone();
                }
                let _ = self.pubsub.send(&resolved_pointer, None).await;
            }
        }
    }

    /// Tries to find the blake3 hash of an immutable pointer by only relying on locally cached
    /// records and without performing any contact with other nodes.
    ///
    /// This can return [`None`] if no local record is found.
    async fn get_blake3_hash(&self, pointer: ImmutablePointer) -> Option<Blake3Hash> {
        let cf = self
            .db
            .cf_handle(URI_TO_B3)
            .expect("No uri_to_b3 column family in resolver db");

        let pointer_bytes = bincode::serialize(&pointer).ok()?;

        let res = self
            .db
            .get_cf(&cf, pointer_bytes)
            .expect("Failed to access db")?;

        bincode::deserialize(&res).ok()
    }

    fn get_origins(&self, hash: Blake3Hash) -> Option<Vec<ResolvedImmutablePointerRecord>> {
        let cf = self
            .db
            .cf_handle(B3_TO_URI)
            .expect("No b3_to_uri column family in resolver db");

        let res = self.db.get_cf(&cf, hash).expect("Failed to access db")?;

        bincode::deserialize(&res).ok()
    }

    fn store_mapping(record: ResolvedImmutablePointerRecord, db: &DB) {
        let b3_hash = record.hash;
        let b3_cf = db
            .cf_handle(B3_TO_URI)
            .expect("No b3_to_uri column family in resolver db");
        let uri_cf = db
            .cf_handle(URI_TO_B3)
            .expect("No uri_to_b3 column family in resolver db");

        let pointer_bytes = bincode::serialize(&record.pointer)
            .expect("Could not serialize pubsub message in resolver");

        let entry = match db.get_cf(&b3_cf, b3_hash).expect("Failed to access db") {
            Some(bytes) => {
                let mut uris: Vec<ResolvedImmutablePointerRecord> = bincode::deserialize(&bytes)
                    .expect("Could not deserialize bytes in rocksdb: resolver");
                if !uris.iter().any(|x| x.pointer == record.pointer) {
                    uris.push(record);
                }
                uris
            },
            None => {
                vec![record]
            },
        };
        db.put_cf(
            &b3_cf,
            b3_hash,
            bincode::serialize(&entry).expect("Failed to serialize payload in resolver"),
        )
        .expect("Failed to insert mapping to db in resolver");
        db.put_cf(&uri_cf, pointer_bytes, b3_hash)
            .expect("Failed to insert mapping to db in resolver")
    }
}
