use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use anyhow::Result;
use fleek_crypto::{AccountOwnerSecretKey, EthAddress, SecretKey};
use futures::future::join_all;
use hp_fixed::unsigned::HpUfixed;
use lightning_application::{Application, ApplicationConfig};
use lightning_broadcast::Broadcast;
use lightning_forwarder::Forwarder;
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{
    Genesis,
    GenesisAccount,
    GenesisNode,
    GenesisPrices,
    GenesisService,
};
use lightning_interfaces::{partial, CheckpointerInterface, Node};
use lightning_notifier::Notifier;
use lightning_pool::{Config as PoolConfig, PoolProvider};
use lightning_rep_collector::ReputationAggregator;
use lightning_signer::Signer;
use lightning_test_utils::json_config::JsonConfigProvider;
use lightning_test_utils::keys::EphemeralKeystore;
use lightning_topology::Topology;
use ready::tokio::TokioReadyWaiter;
use ready::ReadyWaiter;
use tempfile::{tempdir, TempDir};
use tokio::time::sleep;
use types::{ChainId, CommodityTypes, NodePorts, Staking};

use crate::{Checkpointer, CheckpointerConfig};

// #[tokio::test]
// async fn test_checkpointer_start_shutdown() -> Result<()> {
//     let temp_dir = tempdir()?;
//     let _node = TestNode::random(temp_dir.path().to_path_buf().try_into()?).await?;

//     Ok(())
// }

#[tokio::test(flavor = "multi_thread")]
async fn test_checkpointer_over_single_epoch_change() -> Result<()> {
    let _ = tracing_subscriber::fmt::try_init();

    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;

    // sleep(Duration::from_secs(1)).await;

    // Emit epoch changed notification from the first node.
    network.nodes[0]
        .notifier
        .get_emitter()
        // TODO(snormore): Use more realistic values here.
        .epoch_changed(0, [0; 32], [0; 32], [0; 32]);

    sleep(Duration::from_secs(2)).await;

    // Check that the checkpoint header attestation is saved to the node's database.

    // Emit enough headers for a supermajority of attestations.

    // Check that the aggregate checkpoint header is saved to the node's database.

    println!("Shutting down network");
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_over_many_epoch_changes() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_no_supermajority_of_attestations() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_fake_and_corrupt_attestation() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_duplicate_attestations() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_too_few_attestations() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_missing_attestations() -> Result<()> {
    // TODO(snormore): Implement this test.
    Ok(())
}

partial!(TestNodeComponents {
    ApplicationInterface = Application<Self>;
    BroadcastInterface = Broadcast<Self>;
    CheckpointerInterface = Checkpointer<Self>;
    ConfigProviderInterface = JsonConfigProvider;
    ForwarderInterface = Forwarder<Self>;
    KeystoreInterface = EphemeralKeystore<Self>;
    NotifierInterface = Notifier<Self>;
    PoolInterface = PoolProvider<Self>;
    ReputationAggregatorInterface = ReputationAggregator<Self>;
    TopologyInterface = Topology<Self>;
    SignerInterface = Signer<Self>;
});

pub struct TestNetworkBuilder {
    pub num_nodes: usize,
}

impl TestNetworkBuilder {
    pub fn new() -> Self {
        Self { num_nodes: 3 }
    }

    pub fn with_num_nodes(mut self, num_nodes: usize) -> Self {
        self.num_nodes = num_nodes;
        self
    }

    /// Builds a new test network with the given number of nodes, and starts each of them.
    pub async fn build(self) -> Result<TestNetwork> {
        let temp_dir = tempdir()?;

        // Build and start the nodes.
        let mut nodes = join_all(
            (0..self.num_nodes)
                .map(|i| TestNode::build(temp_dir.path().join(format!("node-{}", i)))),
        )
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        // Wait for ready before building genesis.
        join_all(
            nodes
                .iter_mut()
                .map(|node| node.before_genesis_ready.wait()),
        )
        .await;

        // Build genesis.
        let genesis = {
            let mut builder = TestGenesisBuilder::default();
            for node in nodes.iter() {
                println!(
                    "Node ready to build genesis: {:?}",
                    node.before_genesis_ready.state()
                );

                // TODO(snormore): Genesis can get ready state and keystore from the node itself.
                builder = builder.with_node(node);
            }
            builder.build()
        };

        // Apply genesis on each node.
        join_all(
            nodes
                .iter_mut()
                .map(|node| node.app.apply_genesis(genesis.clone())),
        )
        .await;

        // Wait for ready after genesis.
        join_all(nodes.iter_mut().map(|node| node.after_genesis_ready.wait())).await;

        let network = TestNetwork::new(temp_dir, nodes).await?;
        Ok(network)
    }
}

pub struct TestNetwork {
    _temp_dir: TempDir,
    pub nodes: Vec<TestNode>,
}

impl TestNetwork {
    pub async fn new(temp_dir: TempDir, nodes: Vec<TestNode>) -> Result<Self> {
        Ok(Self {
            _temp_dir: temp_dir,
            nodes,
        })
    }

    pub async fn shutdown(&mut self) {
        join_all(self.nodes.iter_mut().map(|node| node.shutdown())).await;
    }
}

pub struct TestNode {
    pub inner: Node<TestNodeComponents>,
    // pub home_dir: PathBuf,
    pub before_genesis_ready: TokioReadyWaiter<TestNodeBeforeGenesisReadyState>,
    pub after_genesis_ready: TokioReadyWaiter<()>,

    pub app: fdi::Ref<Application<TestNodeComponents>>,
    // pub broadcast: fdi::Ref<Broadcast<TestNodeComponents>>,
    // pub checkpointer: fdi::Ref<Checkpointer<TestNodeComponents>>,
    pub keystore: fdi::Ref<EphemeralKeystore<TestNodeComponents>>,
    pub notifier: fdi::Ref<Notifier<TestNodeComponents>>,
    // pub pool: fdi::Ref<PoolProvider<TestNodeComponents>>,
}

#[derive(Clone, Debug)]
pub struct TestNodeBeforeGenesisReadyState {
    pub pool_listen_address: SocketAddr,
}

impl Default for TestNodeBeforeGenesisReadyState {
    fn default() -> Self {
        Self {
            pool_listen_address: "0.0.0.0:0".parse().unwrap(),
        }
    }
}

impl TestNode {
    pub async fn build(home_dir: PathBuf) -> Result<Self> {
        let config = JsonConfigProvider::default()
            .with::<Application<TestNodeComponents>>(ApplicationConfig {
                genesis_path: None,
                db_path: Some(home_dir.join("db").try_into().unwrap()),
                ..Default::default()
            })
            .with::<Checkpointer<TestNodeComponents>>(CheckpointerConfig::default_with_home_dir(
                home_dir.as_path(),
            ))
            .with::<PoolProvider<TestNodeComponents>>(PoolConfig {
                // Specify port 0 to get a random available port.
                address: "0.0.0.0:0".parse().unwrap(),
                ..Default::default()
            });

        let keystore = EphemeralKeystore::<TestNodeComponents>::default();

        let node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default().with(config).with(keystore),
        )?;

        node.start().await;

        // Wait for pool to be ready before building genesis.
        let before_genesis_ready = TokioReadyWaiter::new();
        {
            let pool = node.provider.get::<PoolProvider<TestNodeComponents>>();
            let before_genesis_ready = before_genesis_ready.clone();
            spawn!(
                async move {
                    // Wait for pool to be ready.
                    let pool_state = pool.wait_for_ready().await;
                    let state = TestNodeBeforeGenesisReadyState {
                        pool_listen_address: pool_state.listen_address.unwrap(),
                    };
                    println!("Pool listening on: {}", state.pool_listen_address);

                    // Notify that we are ready.
                    before_genesis_ready.notify(state);
                },
                "TEST-NODE ready watcher"
            );
        }

        // Wait for checkpointer to be ready after genesis.
        let after_genesis_ready = TokioReadyWaiter::new();
        {
            let checkpointer = node.provider.get::<Checkpointer<TestNodeComponents>>();
            let after_genesis_ready = after_genesis_ready.clone();
            spawn!(
                async move {
                    // Wait for checkpointer to be ready.
                    tracing::debug!("waiting for checkpointer to be ready");
                    checkpointer.wait_for_ready().await;
                    tracing::debug!("checkpointer ready");

                    // Notify that we are ready.
                    after_genesis_ready.notify(());
                },
                "TEST-NODE checkpointer ready watcher"
            );
        }

        Ok(Self {
            app: node.provider.get::<Application<TestNodeComponents>>(),
            // checkpointer: node.provider.get::<Checkpointer<TestNodeComponents>>(),
            // broadcast: node.provider.get::<Broadcast<TestNodeComponents>>(),
            keystore: node.provider.get::<EphemeralKeystore<TestNodeComponents>>(),
            notifier: node.provider.get::<Notifier<TestNodeComponents>>(),
            // pool: node.provider.get::<PoolProvider<TestNodeComponents>>(),
            inner: node,
            // home_dir: spec.home_dir,
            before_genesis_ready,
            after_genesis_ready,
        })
    }

    // pub async fn random(home_dir: ResolvedPathBuf) -> Result<Self> {
    //     // Build node spec.
    //     // let mut spec = TestNodeSpec {
    //     //     home_dir: home_dir.to_path_buf(),
    //     //     keystore: EphemeralKeystore::<TestNodeComponents>::default(),
    //     //     genesis_path: PathBuf::default(),
    //     // };

    //     // Build genesis.
    //     let genesis = TestGenesisBuilder::default()
    //         .with_nodes(vec![spec.clone()])
    //         .build();
    //     let genesis_path = genesis.write_to_dir(spec.home_dir.clone().try_into().unwrap())?;

    //     // Update genesis path on spec.
    //     spec.genesis_path = genesis_path.to_path_buf();

    //     // Build and return node.
    //     let node = TestNode::build(spec).await?;
    //     Ok(node)
    // }

    pub async fn start(&mut self) {
        self.inner.start().await;
    }

    pub async fn shutdown(&mut self) {
        self.inner.shutdown().await;
    }
}

#[derive(Clone)]
pub struct TestGenesisBuilder {
    chain_id: ChainId,
    owner_secret_key: AccountOwnerSecretKey,
    protocol_address: EthAddress,
    nodes: Vec<GenesisNode>,
}

impl Default for TestGenesisBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TestGenesisBuilder {
    pub fn new() -> Self {
        let protocol_secret_key = AccountOwnerSecretKey::generate();
        Self {
            chain_id: 1337,
            owner_secret_key: AccountOwnerSecretKey::generate(),
            nodes: Vec::new(),
            protocol_address: protocol_secret_key.to_pk().into(),
        }
    }

    pub fn with_chain_id(self, chain_id: ChainId) -> Self {
        Self { chain_id, ..self }
    }

    pub fn with_owner(self, secret_key: AccountOwnerSecretKey) -> Self {
        Self {
            owner_secret_key: secret_key,
            ..self
        }
    }

    pub fn with_protocol_address(self, address: EthAddress) -> Self {
        Self {
            protocol_address: address,
            ..self
        }
    }

    pub fn with_node(mut self, node: &TestNode) -> Self {
        let node_secret_key = node.keystore.get_ed25519_sk();
        let node_public_key = node_secret_key.to_pk();
        let consensus_secret_key = node.keystore.get_bls_sk();
        let consensus_public_key = consensus_secret_key.to_pk();
        let node_domain = "127.0.0.1".parse().unwrap();
        let ready = node.before_genesis_ready.state().expect("node not ready");
        let ports = NodePorts {
            pool: ready.pool_listen_address.port(),
            ..Default::default()
        };

        self.nodes.push(GenesisNode::new(
            self.owner_secret_key.to_pk().into(),
            node_public_key,
            node_domain,
            consensus_public_key,
            node_domain,
            node_public_key,
            ports,
            Some(Staking {
                staked: HpUfixed::<18>::from(1000u32),
                stake_locked_until: 0,
                locked: HpUfixed::<18>::zero(),
                locked_until: 0,
            }),
            true,
        ));

        self
    }

    // pub fn with_nodes(self, nodes: Vec<TestNodeSpec>) -> Self {
    //     let nodes = nodes
    //         .into_iter()
    //         .map(|node| {
    //             let node_secret_key = node.keystore.get_ed25519_sk();
    //             let node_public_key = node_secret_key.to_pk();
    //             let consensus_secret_key = node.keystore.get_bls_sk();
    //             let consensus_public_key = consensus_secret_key.to_pk();
    //             let node_domain = "127.0.0.1".parse().unwrap();

    //             GenesisNode::new(
    //                 self.owner_secret_key.to_pk().into(),
    //                 node_public_key,
    //                 node_domain,
    //                 consensus_public_key,
    //                 node_domain,
    //                 node_public_key,
    //                 // TODO(snormore): These should be random ports.
    //                 NodePorts::default(),
    //                 Some(Staking {
    //                     staked: HpUfixed::<18>::from(1000u32),
    //                     stake_locked_until: 0,
    //                     locked: HpUfixed::<18>::zero(),
    //                     locked_until: 0,
    //                 }),
    //                 true,
    //             )
    //         })
    //         .collect();

    //     Self { nodes, ..self }
    // }

    pub fn build(self) -> Genesis {
        Genesis {
            chain_id: self.chain_id,
            epoch_start: 1684276288383,
            epoch_time: 120000,
            committee_size: 10,
            node_count: 10,
            min_stake: 1000,
            eligibility_time: 1,
            lock_time: 5,
            protocol_share: 0,
            node_share: 80,
            service_builder_share: 20,
            max_inflation: 10,
            consumer_rebate: 0,
            max_boost: 4,
            max_lock_time: 1460,
            supply_at_genesis: 1000000,
            min_num_measurements: 2,
            protocol_fund_address: self.protocol_address,
            governance_address: self.protocol_address,
            node_info: self.nodes,
            service: vec![
                GenesisService {
                    id: 0,
                    owner: EthAddress::from_str("0xDC0A31F9eeb151f82BF1eE6831095284fC215Ee7")
                        .unwrap(),
                    commodity_type: CommodityTypes::Bandwidth,
                },
                GenesisService {
                    id: 1,
                    owner: EthAddress::from_str("0x684166BDbf530a256d7c92Fa0a4128669aFd9B9F")
                        .unwrap(),
                    commodity_type: CommodityTypes::Compute,
                },
            ],
            account: vec![GenesisAccount {
                public_key: self.owner_secret_key.to_pk().into(),
                flk_balance: HpUfixed::<18>::from(100690000000000000000u128),
                stables_balance: 100,
                bandwidth_balance: 100,
            }],
            client: HashMap::new(),
            commodity_prices: vec![
                GenesisPrices {
                    commodity: CommodityTypes::Bandwidth,
                    price: 0.1,
                },
                GenesisPrices {
                    commodity: CommodityTypes::Compute,
                    price: 0.2,
                },
            ],
            total_served: HashMap::new(),
            latencies: None,
        }
    }
}
