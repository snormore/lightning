use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use affair::AsyncWorker;
use anyhow::{anyhow, Result};
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{ChainId, NodeInfo};
use lightning_interfaces::{spawn_worker, GenesisApplierInterface};
use tracing::{error, info};

use crate::config::{Config, StorageConfig};
use crate::env::{ApplicationEnv, Env, UpdateWorker};
use crate::state::QueryRunner;
pub struct Application<C: Collection> {
    apply_genesis: ApplicationGenesisApplier,
    update_socket: Mutex<Option<ExecutionEngineSocket>>,
    query_runner: QueryRunner,
    collection: PhantomData<C>,
}

impl<C: Collection> Application<C> {
    /// Create a new instance of the application layer using the provided configuration.
    fn init(
        config: &C::ConfigProviderInterface,
        blockstore: &C::BlockstoreInterface,
        fdi::Cloned(waiter): fdi::Cloned<ShutdownWaiter>,
    ) -> Result<Self> {
        let config = config.get::<Self>();
        if let StorageConfig::RocksDb = &config.storage {
            assert!(
                config.db_path.is_some(),
                "db_path must be specified for RocksDb backend"
            );
        }

        let mut env = Env::new(&config, None).expect("Failed to initialize environment.");

        // Auto-apply genesis by default; disabled only if `dev.auto_apply_genesis` is false.
        match &config.dev {
            Some(dev) => {
                if dev.auto_apply_genesis {
                    env.apply_genesis_block(&config)?;
                }
            },
            None => {
                env.apply_genesis_block(&config)?;
            },
        }

        let query_runner = env.query_runner();
        let env = Arc::new(tokio::sync::Mutex::new(env));
        let worker = UpdateWorker::<C>::new(env.clone(), blockstore.clone());
        let update_socket = spawn_worker!(worker, "APPLICATION", waiter, crucial);

        Ok(Self {
            apply_genesis: ApplicationGenesisApplier::new(config, env),
            query_runner,
            update_socket: Mutex::new(Some(update_socket)),
            collection: PhantomData,
        })
    }
}

impl<C: Collection> ConfigConsumer for Application<C> {
    const KEY: &'static str = "application";

    type Config = Config;
}

impl<C: Collection> fdi::BuildGraph for Application<C> {
    fn build_graph() -> fdi::DependencyGraph {
        fdi::DependencyGraph::new().with(Self::init)
    }
}

impl<C: Collection> ApplicationInterface<C> for Application<C> {
    /// The type for the sync query executor.
    type SyncExecutor = QueryRunner;

    /// The type for applying a genesis block to the state.
    type GenesisApplier = ApplicationGenesisApplier;

    /// Returns a socket that should be used to submit transactions to be executed
    /// by the application layer.
    ///
    /// # Safety
    ///
    /// See the safety document for the [`ExecutionEngineSocket`].
    fn transaction_executor(&self) -> ExecutionEngineSocket {
        self.update_socket
            .lock()
            .unwrap()
            .take()
            .expect("Execution Engine Socket has already been taken")
    }

    /// Returns the instance of a sync query runner which can be used to run queries without
    /// blocking or awaiting. A naive (& blocking) implementation can achieve this by simply
    /// putting the entire application state in an `Arc<RwLock<T>>`, but that is not optimal
    /// and is the reason why we have `Atomo` to allow us to have the same kind of behavior
    /// without slowing down the system.
    fn sync_query(&self) -> Self::SyncExecutor {
        self.query_runner.clone()
    }

    /// Returns the instance of a genesis applier which can be used to apply a genesis block to the
    /// state.
    ///
    /// This contains a reference to env wrapped in Arc/Mutex, so it is safe to clone and use in
    /// other components, such as the RPC components.
    fn genesis_applier(&self) -> Self::GenesisApplier {
        self.apply_genesis.clone()
    }

    async fn load_from_checkpoint(
        config: &Config,
        checkpoint: Vec<u8>,
        checkpoint_hash: [u8; 32],
    ) -> Result<()> {
        // Due to a race condition on shutdowns when a node checkpoints, we should sleep and try
        // again if there is a lock on the DB at this stage of the process
        let mut counter = 0;

        loop {
            match ApplicationEnv::new(config, Some((checkpoint_hash, &checkpoint))) {
                Ok(mut env) => {
                    info!(
                        "Successfully built database from checkpoint with hash {checkpoint_hash:?}"
                    );

                    // Update the last epoch hash on state
                    env.update_last_epoch_hash(checkpoint_hash)?;

                    return Ok(());
                },
                Err(e) => {
                    if counter > 10 {
                        error!("Failed to build app db from checkpoint: {e:?}");
                        return Err(anyhow!("Failed to build app db from checkpoint: {}", e));
                    } else {
                        counter += 1;
                        tokio::time::sleep(Duration::from_secs(3)).await;
                    }
                },
            }
        }
    }

    fn get_chain_id(config: &Config) -> Result<ChainId> {
        Ok(config.genesis()?.chain_id)
    }

    fn get_genesis_committee(config: &Config) -> Result<Vec<NodeInfo>> {
        Ok(config
            .genesis()?
            .node_info
            .iter()
            .filter(|node| node.genesis_committee)
            .map(NodeInfo::from)
            .collect())
    }

    /// Resets the state tree by clearing it and rebuilding it from the full state.
    ///
    /// This method is unsafe because it acts directly on the underlying storage backend.
    fn reset_state_tree_unsafe(config: &Config) -> Result<()> {
        let mut env = ApplicationEnv::new(config, None)?;
        env.inner.reset_state_tree_unsafe()
    }
}

#[derive(Clone)]
pub struct ApplicationGenesisApplier {
    config: Config,
    env: Arc<tokio::sync::Mutex<ApplicationEnv>>,
}

impl ApplicationGenesisApplier {
    pub fn new(config: Config, env: Arc<tokio::sync::Mutex<ApplicationEnv>>) -> Self {
        Self { config, env }
    }
}

impl GenesisApplierInterface for ApplicationGenesisApplier {
    /// Apply genesis block to the application state, if not already applied.
    fn apply_genesis(&self) -> Result<bool> {
        // TODO(snormore): Figure out if this is ok to do.
        futures::executor::block_on(async {
            self.env.lock().await.apply_genesis_block(&self.config)
        })
    }
}
