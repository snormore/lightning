use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use fleek_crypto::{AccountOwnerSecretKey, SecretKey};
use lightning_application::{Application, ApplicationConfig};
use lightning_blockstore::blockstore::Blockstore;
use lightning_blockstore::config::Config as BlockstoreConfig;
use lightning_checkpointer::{Checkpointer, CheckpointerConfig, CheckpointerDatabaseConfig};
use lightning_committee_beacon::{
    CommitteeBeaconComponent,
    CommitteeBeaconConfig,
    CommitteeBeaconDatabaseConfig,
};
use lightning_interfaces::prelude::*;
use lightning_node::ContainedNode;
use lightning_pool::{Config as PoolConfig, PoolProvider};
use lightning_rep_collector::MyReputationReporter;
use lightning_rpc::config::Config as RpcConfig;
use lightning_rpc::Rpc;
use lightning_signer::Signer;
use lightning_utils::config::TomlConfigProvider;
use ready::tokio::TokioReadyWaiter;
use ready::ReadyWaiter;
use tempfile::tempdir;

use super::{BoxedNode, SyncBroadcaster, TestNode, TestNodeBeforeGenesisReadyState};
use crate::consensus::{MockConsensus, MockConsensusGroup, MockForwarder};
use crate::keys::EphemeralKeystore;

#[derive(Clone)]
pub struct TestNodeBuilder {
    home_dir: Option<PathBuf>,
    use_mock_consensus: bool,
    mock_consensus_group: Option<MockConsensusGroup>,
}

impl TestNodeBuilder {
    pub fn new() -> Self {
        Self {
            home_dir: None,
            use_mock_consensus: true,
            mock_consensus_group: None,
        }
    }

    pub fn with_home_dir(mut self, home_dir: PathBuf) -> Self {
        self.home_dir = Some(home_dir);
        self
    }

    pub fn with_mock_consensus(mut self, mock_consensus_group: Option<MockConsensusGroup>) -> Self {
        self.use_mock_consensus = true;
        self.mock_consensus_group = mock_consensus_group;
        self
    }

    pub fn without_mock_consensus(mut self) -> Self {
        self.use_mock_consensus = false;
        self.mock_consensus_group = None;
        self
    }

    pub async fn build<C: NodeComponents>(self) -> Result<BoxedNode> {
        let (temp_dir, home_dir) = if let Some(home_dir) = self.home_dir {
            (None, home_dir)
        } else {
            let temp_dir = tempdir()?;
            let home_dir = temp_dir.path().to_path_buf();
            (Some(temp_dir), home_dir)
        };

        let config = TomlConfigProvider::<C>::new();

        // Configure application component.
        config.inject::<Application<C>>(ApplicationConfig {
            genesis_path: None,
            db_path: Some(home_dir.join("app").try_into().unwrap()),
            ..Default::default()
        });

        // Configure blockstore component.
        config.inject::<Blockstore<C>>(BlockstoreConfig {
            root: home_dir.join("blockstore").try_into().unwrap(),
        });

        // Configure checkpointer component.
        config.inject::<Checkpointer<C>>(CheckpointerConfig {
            database: CheckpointerDatabaseConfig {
                path: home_dir.join("checkpointer").try_into().unwrap(),
            },
        });

        // Configure committee beacon component.
        config.inject::<CommitteeBeaconComponent<C>>(CommitteeBeaconConfig {
            database: CommitteeBeaconDatabaseConfig {
                path: home_dir.join("committee-beacon").try_into().unwrap(),
            },
        });

        // Configure consensus component.
        if self.use_mock_consensus {
            config.inject::<MockConsensus<C>>(
                self.mock_consensus_group
                    .as_ref()
                    .map(|group| group.config.clone())
                    .unwrap_or_default(),
            );
        }

        // Configure pool component.
        config.inject::<PoolProvider<C>>(PoolConfig {
            // Specify port 0 to get a random available port.
            address: "0.0.0.0:0".parse().unwrap(),
            ..Default::default()
        });

        // Configure RPC component.
        config.inject::<Rpc<C>>(RpcConfig {
            // Specify port 0 to get a random available port.
            addr: "0.0.0.0:0".parse().unwrap(),
            hmac_secret_dir: Some(home_dir.clone()),
            ..Default::default()
        });

        // Configure keystore component.
        config.inject::<EphemeralKeystore<C>>(Default::default());

        // Initialize the node.
        let provider = fdi::MultiThreadedProvider::default().with(config);
        if let Some(mock_consensus_group) = self.mock_consensus_group {
            provider.insert(mock_consensus_group);
        }
        let node = ContainedNode::<C>::new(provider, None);

        // Start the node.
        tokio::time::timeout(Duration::from_secs(15), node.spawn()).await???;

        // Wait for the node to be ready.
        let shutdown = node.shutdown_waiter();

        // Wait for components to be ready before building genesis.
        let before_genesis_ready = TokioReadyWaiter::new();
        {
            let pool = node.provider().get::<PoolProvider<C>>();
            let rpc = node.provider().get::<Rpc<C>>();
            let before_genesis_ready = before_genesis_ready.clone();
            let shutdown = shutdown.clone();
            spawn!(
                async {
                    tokio::time::timeout(Duration::from_secs(15), async move {
                        // Wait for pool to be ready.
                        let pool_state = pool.wait_for_ready().await;

                        // Wait for rpc to be ready.
                        let rpc_state = rpc.wait_for_ready().await;

                        // Notify that we are ready.
                        let state = TestNodeBeforeGenesisReadyState {
                            pool_listen_address: pool_state.listen_address.unwrap(),
                            rpc_listen_address: rpc_state.listen_address,
                        };
                        before_genesis_ready.notify(state);
                    })
                    .await
                    .unwrap();
                },
                "TEST-NODE before genesis ready watcher",
                crucial(shutdown)
            );
        }

        // Wait for components to be ready after genesis.
        let after_genesis_ready = TokioReadyWaiter::new();
        {
            let app_query = node.provider().get::<Application<C>>().sync_query();
            let checkpointer = node.provider().get::<Checkpointer<C>>();
            let after_genesis_ready = after_genesis_ready.clone();
            let shutdown = shutdown.clone();
            spawn!(
                async {
                    tokio::time::timeout(Duration::from_secs(15), async move {
                        // Wait for genesis to be applied.
                        app_query.wait_for_genesis().await;

                        // Wait for the checkpointer to be ready.
                        checkpointer.wait_for_ready().await;

                        // Notify that we are ready.
                        after_genesis_ready.notify(());
                    })
                    .await
                    .unwrap();
                },
                "TEST-NODE after genesis ready watcher",
                crucial(shutdown)
            );
        }

        let app = node.provider().get::<Application<C>>();
        Ok(Box::new(TestNode {
            app_query: node
                .provider()
                .get::<c!(C::ApplicationInterface::SyncExecutor)>(),
            app,
            broadcast: node.provider().get::<SyncBroadcaster<C>>(),
            checkpointer: node.provider().get::<Checkpointer<C>>(),
            notifier: node
                .provider()
                .get::<<C as NodeComponents>::NotifierInterface>(),
            forwarder: node.provider().get::<MockForwarder<C>>(),
            keystore: node.provider().get::<EphemeralKeystore<C>>(),
            pool: node.provider().get::<PoolProvider<C>>(),
            rpc: node.provider().get::<Rpc<C>>(),
            reputation_reporter: node.provider().get::<MyReputationReporter>(),
            signer: node.provider().get::<Signer<C>>(),

            inner: node,
            before_genesis_ready,
            after_genesis_ready,
            temp_dir,
            home_dir,
            owner_secret_key: AccountOwnerSecretKey::generate(),
        }))
    }
}
