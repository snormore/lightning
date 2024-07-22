use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use affair::AsyncWorkerUnordered;
use anyhow::Result;
use fleek_crypto::{NodePublicKey, NodeSecretKey};
use futures::stream::FuturesUnordered;
use futures::{AsyncReadExt, StreamExt, TryStreamExt};
use lightning_interfaces::prelude::*;
use lightning_interfaces::{spawn_worker, RequestHeader, RequesterInterface, TaskError};
use lightning_metrics::increment_counter;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use schema::task_broker::{TaskRequest, TaskResponse, TaskScope};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use tokio::time::timeout;
use tracing::{debug, warn};
use types::NodeIndex;

use crate::local::{LocalTaskSocket, LocalTaskWorker};

pub struct TaskBroker<C: Collection> {
    socket: Arc<RwLock<Option<LocalTaskSocket>>>,
    requester: Arc<<C::PoolInterface as PoolInterface<C>>::Requester>,
    topology: tokio::sync::watch::Receiver<Arc<Vec<Vec<NodePublicKey>>>>,
    query_runner: c!(C::ApplicationInterface::SyncExecutor),
    max_depth: u8,
    request_timeout: Duration,
    temp: Option<RequestWorkerInner<C>>,
}

struct RequestWorkerInner<C: Collection> {
    sk: NodeSecretKey,
    responder: c!(C::PoolInterface::Responder),
    config: TaskBrokerConfig,
}

impl<C: Collection> Clone for TaskBroker<C> {
    fn clone(&self) -> Self {
        Self {
            socket: self.socket.clone(),
            requester: self.requester.clone(),
            topology: self.topology.clone(),
            query_runner: self.query_runner.clone(),
            max_depth: self.max_depth,
            request_timeout: self.request_timeout,
            temp: None,
        }
    }
}

impl<C: Collection> TaskBroker<C> {
    fn init(
        config: &C::ConfigProviderInterface,
        keystore: &C::KeystoreInterface,
        topology: &C::TopologyInterface,
        pool: &C::PoolInterface,
        fdi::Cloned(query_runner): fdi::Cloned<c!(C::ApplicationInterface::SyncExecutor)>,
    ) -> Result<Self> {
        let config @ TaskBrokerConfig {
            max_depth,
            request_timeout,
            ..
        } = config.get::<Self>();

        let (req, responder) = pool.open_req_res(lightning_interfaces::ServiceScope::TaskBroker);

        Ok(Self {
            socket: Arc::new(RwLock::new(None)),
            requester: req.into(),
            topology: topology.get_receiver(),
            query_runner,
            max_depth,
            request_timeout,
            temp: Some(RequestWorkerInner {
                sk: keystore.get_ed25519_sk(),
                responder,
                config,
            }),
        })
    }

    fn post_init(
        &mut self,
        service_executor: &C::ServiceExecutorInterface,
        fdi::Cloned(shutdown): fdi::Cloned<ShutdownWaiter>,
    ) {
        let RequestWorkerInner {
            sk,
            responder,
            config,
        } = self.temp.take().unwrap();

        // spawn worker to handle executing local tasks
        let provider = service_executor.get_provider();
        let waiter = shutdown.clone();
        let socket = spawn_worker!(
            LocalTaskWorker::new(provider, config.max_tasks),
            "TASK BROKER: Local task worker",
            waiter
        );

        *self.socket.write().unwrap() = Some(socket.clone());

        RequestWorker::<C>::spawn(
            sk,
            responder,
            socket.clone(),
            config.max_peer_tasks,
            shutdown,
        );
    }

    /// Get current topology cluster
    fn get_cluster(&self) -> Result<Vec<NodePublicKey>, TaskError> {
        let topology = self.topology.borrow();
        let Some(nodes) = topology.first() else {
            // TODO(oz): should we attempt to wait for topology to change and
            //           recover from the error here?
            return Err(TaskError::Internal("topology not initialized".to_string()));
        };
        if nodes.is_empty() {
            return Err(TaskError::Internal("no nodes in cluster".into()));
        }
        Ok(nodes.clone())
    }

    fn get_random_cluster_node(&self) -> Result<u32, TaskError> {
        // Select a random node within the local cluster
        let cluster = self.get_cluster()?;
        let pub_key = cluster.choose(&mut thread_rng()).unwrap();
        let idx = self
            .query_runner
            .pubkey_to_index(pub_key)
            .expect("topology should never ever give unknown node pubkeys/indecies");
        Ok(idx)
    }

    async fn run_task_on_peer(
        &self,
        task: TaskRequest,
        peer: NodeIndex,
    ) -> Result<TaskResponse, TaskError> {
        debug!("Running task on peer {peer}");

        // Encode task and send the request
        let mut buf = Vec::new();
        task.encode(&mut buf)
            .map_err(|e| TaskError::Internal(e.to_string()))?;

        match timeout(
            self.request_timeout,
            self.requester.request(peer, buf.into()),
        )
        .await
        {
            Ok(Ok(res)) => {
                // Stream and decode response
                let mut res = res.body().into_async_read();
                let mut buf = Vec::new();
                res.read_to_end(&mut buf)
                    .await
                    .map_err(|_| TaskError::InvalidResponse)?;

                TaskResponse::decode(&buf).map_err(|_| TaskError::InvalidResponse)
            },
            Ok(Err(e)) => {
                warn!("Task on peer {peer} failed: {e}");
                Err(TaskError::Connect)
            },
            Err(_) => {
                warn!("Task on peer {peer} timed out");
                Err(TaskError::Timeout)
            },
        }
    }

    /// Get a random peer and run the task on it
    async fn run_single_task(&self, task: TaskRequest) -> Result<TaskResponse, TaskError> {
        let idx = self.get_random_cluster_node()?;
        self.run_task_on_peer(task, idx).await
    }

    /// Get peers in the cluster and run the task on them
    /// TODO: Collect 2/3 of the same responses and respond early, pruning invalid ones.
    async fn run_cluster_task(&self, task: TaskRequest) -> Vec<Result<TaskResponse, TaskError>> {
        match self.get_cluster() {
            Ok(c) => {
                let n = c.len();
                let min = (2. * n as f32 / 3.).ceil() as usize;

                let mut futs = c
                    .iter()
                    .map(|v| {
                        self.query_runner
                            .pubkey_to_index(v)
                            .expect("topology should never give unknown node pubkeys/indecies")
                    })
                    .map(|idx| self.run_task_on_peer(task.clone(), idx))
                    .collect::<FuturesUnordered<_>>();

                let mut responses = HashMap::<fleek_blake3::Hash, Vec<TaskResponse>>::new();
                let mut errors = Vec::new();
                while let Some(result) = futs.next().await {
                    match result {
                        Ok(response) => {
                            let hash = fleek_blake3::hash(&response.payload);
                            let entry = responses.entry(hash).or_default();
                            entry.push(response);
                            let len = entry.len();

                            if len >= min {
                                // We have reached consensus on >=2/3 responses.
                                // Collect them, chain any errors onto the end, and return.
                                return responses
                                    .remove(&hash)
                                    .unwrap()
                                    .into_iter()
                                    .map(Ok)
                                    .chain(errors.into_iter().map(Err))
                                    .collect();
                            }
                        },
                        Err(e) => errors.push(e),
                    }
                }

                errors.into_iter().map(Err).collect()
            },
            Err(e) => vec![Err(e)],
        }
    }
}

impl<C: Collection> fdi::BuildGraph for TaskBroker<C> {
    fn build_graph() -> fdi::DependencyGraph {
        fdi::DependencyGraph::default()
            .with(Self::init.with_event_handler("_post", Self::post_init))
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct TaskBrokerConfig {
    // Maximum depth of task recursion allowed
    pub max_depth: u8,
    // Maximum number of peer requested tasks allowed to run concurrently
    pub max_peer_tasks: usize,
    // Maximum number of all tasks allowed to run concurrently
    pub max_tasks: usize,
    // Request timeout
    #[serde(with = "humantime_serde")]
    pub request_timeout: Duration,
}
impl Default for TaskBrokerConfig {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_peer_tasks: 128,
            max_tasks: 256,
            request_timeout: Duration::from_secs(30),
        }
    }
}

impl<C: Collection> ConfigConsumer for TaskBroker<C> {
    const KEY: &'static str = "task_broker";
    type Config = TaskBrokerConfig;
}

impl<C: Collection> TaskBrokerInterface<C> for TaskBroker<C> {
    async fn run(
        &self,
        depth: u8,
        scope: TaskScope,
        task: TaskRequest,
    ) -> Vec<Result<TaskResponse, TaskError>> {
        if depth > self.max_depth {
            increment_counter!(
                "task_broker_task_blocked",
                Some("Number of task requests blocked due to reaching max depth")
            );
            let err = TaskError::MaxDepth(self.max_depth);
            warn!("{err}");
            return vec![Err(err)];
        }
        match scope {
            TaskScope::Local => {
                let socket = self
                    .socket
                    .read()
                    .expect("failed to access")
                    .as_ref()
                    .unwrap()
                    .clone();

                let res = socket
                    .run((depth, task))
                    .await
                    .expect("failed to run task")
                    .map_err(|e| {
                        task_failed_metric(scope);
                        TaskError::Internal(e.to_string())
                    });
                vec![res]
            },
            TaskScope::Single => {
                let res = self.run_single_task(task).await;
                if let Err(e) = &res {
                    warn!("Failed to run task on peer: {e}");
                    task_failed_metric(scope);
                }
                vec![res]
            },
            TaskScope::Cluster => self.run_cluster_task(task).await,
            TaskScope::Multicluster(_) => {
                unimplemented!("Multicluster task consensus not implemented")
            },
        }
    }
}

fn task_failed_metric(scope: TaskScope) {
    let scope = scope.to_string();
    increment_counter!("task_broker_request_failed", Some("Task broker request failures per scope"), "scope" => scope.as_str())
}

/// Worker for handling incoming tasks
pub struct RequestWorker<C: Collection> {
    // Receive incoming tasks
    responder: c!(C::PoolInterface::Responder),
    // For executing tasks we receive locally
    socket: LocalTaskSocket,
    // Limit number of concurrent incoming requests
    semaphore: Arc<Semaphore>,
    // Pending tasks we spawn
    pending_tasks: JoinSet<()>,
}

impl<C: Collection> RequestWorker<C> {
    pub fn spawn(
        _sk: NodeSecretKey,
        responder: c!(C::PoolInterface::Responder),
        socket: LocalTaskSocket,
        max_peer_tasks: usize,
        shutdown: ShutdownWaiter,
    ) {
        // Hack to ensure the task set is never empty
        let mut set = JoinSet::new();
        set.spawn(futures::future::pending());

        let fut = Self {
            responder,
            socket,
            pending_tasks: set,
            semaphore: Arc::new(Semaphore::new(max_peer_tasks)),
        }
        .run();

        spawn!(
            async move { shutdown.run_until_shutdown(fut).await },
            "TASK BROKER: Incoming task loop"
        );
    }

    async fn run(mut self) {
        loop {
            tokio::select! {
                // Drive pending tasks forward
                _ = self.pending_tasks.join_next() => {},
                // Incoming direct task requests
               Ok((header, responder)) = self.responder.get_next_request() => {
                    if let Err(e) = self.handle_incoming_pool_task(header, responder) {
                        warn!("Failed to handle incoming pool task: {e}");
                    }
                },
                else => {
                    break;
                }
            }
        }
    }

    fn handle_incoming_pool_task(
        &mut self,
        header: RequestHeader,
        mut handle: impl lightning_interfaces::RequestInterface,
    ) -> Result<()> {
        // Parse request payload
        let request = TaskRequest::decode(&header.bytes)?;

        let socket = self.socket.clone();
        let lock = self.semaphore.clone();
        self.pending_tasks.spawn(async move {
            // wait our turn before running
            let _ = lock.acquire().await;
            match socket.run((0, request)).await {
                Ok(Ok(res)) => {
                    let mut bytes = Vec::new();
                    res.encode(&mut bytes).expect("failed to encode response");
                    if let Err(e) = handle.send(bytes.into()).await {
                        warn!("Failed to send task response: {e}");
                    }
                },
                Ok(Err(e)) => {
                    warn!("Failed to run task: {e}");
                    handle.reject(types::RejectReason::Other)
                },
                Err(e) => warn!("Failed to run task, socket failed: {e}"),
            };
        });

        Ok(())
    }
}
