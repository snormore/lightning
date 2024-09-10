use std::path::PathBuf;

use anyhow::Result;
use lightning_application::{Application, ApplicationConfig};
use lightning_broadcast::Broadcast;
use lightning_interfaces::prelude::*;
use lightning_notifier::Notifier;
use lightning_pool::{Config as PoolConfig, PoolProvider};
use lightning_rpc::config::Config as RpcConfig;
use lightning_rpc::Rpc;
use ready::tokio::TokioReadyWaiter;
use ready::ReadyWaiter;

use super::{TestNode, TestNodeBeforeGenesisReadyState, TestNodeComponents};
use crate::json_config::JsonConfigProvider;
use crate::keys::EphemeralKeystore;

pub struct TestNodeBuilder {
    home_dir: PathBuf,
}

impl TestNodeBuilder {
    pub fn new(home_dir: PathBuf) -> Self {
        Self { home_dir }
    }

    pub async fn build(self) -> Result<TestNode> {
        let config = JsonConfigProvider::default()
            .with::<Application<TestNodeComponents>>(ApplicationConfig {
                genesis_path: None,
                db_path: Some(self.home_dir.join("app").try_into().unwrap()),
                ..Default::default()
            })
            .with::<PoolProvider<TestNodeComponents>>(PoolConfig {
                // Specify port 0 to get a random available port.
                address: "0.0.0.0:0".parse().unwrap(),
                ..Default::default()
            })
            .with::<Rpc<TestNodeComponents>>(RpcConfig {
                // Specify port 0 to get a random available port.
                addr: "0.0.0.0:0".parse().unwrap(),
                hmac_secret_dir: Some(self.home_dir.clone()),
                ..Default::default()
            });

        let keystore = EphemeralKeystore::<TestNodeComponents>::default();

        let node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default().with(config).with(keystore),
        )?;

        // Start the node.
        node.start().await;

        let shutdown = node
            .shutdown_waiter()
            .expect("node missing shutdown waiter");

        // Wait for pool to be ready before building genesis.
        let before_genesis_ready = TokioReadyWaiter::new();
        {
            let pool = node.provider.get::<PoolProvider<TestNodeComponents>>();
            let rpc = node.provider.get::<Rpc<TestNodeComponents>>();
            let before_genesis_ready = before_genesis_ready.clone();
            let shutdown = shutdown.clone();
            spawn!(
                async move {
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
                },
                "TEST-NODE before genesis ready watcher",
                crucial(shutdown)
            );
        }

        // Wait for checkpointer to be ready after genesis.
        let after_genesis_ready = TokioReadyWaiter::new();
        {
            let after_genesis_ready = after_genesis_ready.clone();
            let shutdown = shutdown.clone();
            spawn!(
                async move {
                    // Notify that we are ready.
                    after_genesis_ready.notify(());
                },
                "TEST-NODE after genesis ready watcher",
                crucial(shutdown)
            );
        }

        Ok(TestNode {
            app: node.provider.get::<Application<TestNodeComponents>>(),
            keystore: node.provider.get::<EphemeralKeystore<TestNodeComponents>>(),
            notifier: node.provider.get::<Notifier<TestNodeComponents>>(),
            pool: node.provider.get::<PoolProvider<TestNodeComponents>>(),
            broadcast: node.provider.get::<Broadcast<TestNodeComponents>>(),
            rpc: node.provider.get::<Rpc<TestNodeComponents>>(),

            inner: node,
            before_genesis_ready,
            after_genesis_ready,
            home_dir: self.home_dir.clone(),
        })
    }
}
