use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use fleek_crypto::{AccountOwnerSecretKey, SecretKey};
use lightning_application::app::Application;
use lightning_application::config::{Config as AppConfig, Mode, StorageConfig};
use lightning_application::genesis::{Genesis, GenesisNode};
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{DeliveryAcknowledgment, DeliveryAcknowledgmentProof, NodePorts};
use lightning_notifier::Notifier;
use lightning_signer::Signer;
use lightning_test_utils::consensus::{Config as ConsensusConfig, MockConsensus, MockForwarder};
use lightning_test_utils::json_config::JsonConfigProvider;
use lightning_test_utils::keys::EphemeralKeystore;
use lightning_utils::application::QueryRunnerExt;

use crate::{Config, DeliveryAcknowledgmentAggregator};

partial!(TestBinding {
    ConfigProviderInterface = JsonConfigProvider;
    KeystoreInterface = EphemeralKeystore<Self>;
    ApplicationInterface = Application<Self>;
    NotifierInterface = Notifier<Self>;
    SignerInterface = Signer<Self>;
    ForwarderInterface = MockForwarder<Self>;
    ConsensusInterface = MockConsensus<Self>;
    DeliveryAcknowledgmentAggregatorInterface = DeliveryAcknowledgmentAggregator<Self>;
});

async fn init_aggregator(path: PathBuf) -> Node<TestBinding> {
    let keystore = EphemeralKeystore::<TestBinding>::default();
    let (consensus_secret_key, node_secret_key) =
        (keystore.get_bls_sk(), keystore.get_ed25519_sk());
    let node_public_key = node_secret_key.to_pk();
    let consensus_public_key = consensus_secret_key.to_pk();
    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner_public_key = owner_secret_key.to_pk();

    let mut genesis = Genesis::load().unwrap();
    genesis.node_info = vec![GenesisNode::new(
        owner_public_key.into(),
        node_public_key,
        "127.0.0.1".parse().unwrap(),
        consensus_public_key,
        "127.0.0.1".parse().unwrap(),
        node_public_key,
        NodePorts {
            primary: 48000,
            worker: 48101,
            mempool: 48102,
            rpc: 48103,
            pool: 48104,
            pinger: 48106,
            handshake: Default::default(),
        },
        None,
        true,
    )];

    let epoch_start = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    genesis.epoch_start = epoch_start;
    genesis.epoch_time = 4000; // millis

    Node::<TestBinding>::init_with_provider(
        fdi::Provider::default()
            .with(
                JsonConfigProvider::default()
                    .with::<Application<TestBinding>>(AppConfig {
                        genesis: Some(genesis),
                        mode: Mode::Test,
                        testnet: false,
                        storage: StorageConfig::InMemory,
                        db_path: None,
                        db_options: None,
                    })
                    .with::<MockConsensus<TestBinding>>(ConsensusConfig {
                        min_ordering_time: 0,
                        max_ordering_time: 1,
                        probability_txn_lost: 0.0,
                        transactions_to_lose: HashSet::new(),
                        new_block_interval: Duration::from_secs(5),
                    })
                    .with::<DeliveryAcknowledgmentAggregator<TestBinding>>(Config {
                        submit_interval: Duration::from_secs(1),
                        db_path: path.try_into().unwrap(),
                    }),
            )
            .with(keystore),
    )
    .unwrap()
}

#[tokio::test]
async fn test_shutdown_and_start_again() {
    let path = std::env::temp_dir().join("lightning-test-dack-aggregator-shutdown");

    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }

    let mut node = init_aggregator(path.clone()).await;

    node.start().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    node.shutdown().await;

    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
}

#[tokio::test]
async fn test_submit_dack() {
    let path = std::env::temp_dir().join("lightning-test-dack-aggregator-submit");

    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }

    let mut node = init_aggregator(path.clone()).await;
    node.start().await;
    tokio::time::sleep(Duration::from_secs(1)).await;

    let query_runner = node
        .provider
        .get::<c!(TestBinding::ApplicationInterface::SyncExecutor)>();

    let socket = node
        .provider
        .get::<DeliveryAcknowledgmentAggregator<TestBinding>>()
        .socket();

    let service_id = 0;
    let commodity = 10;
    let dack = DeliveryAcknowledgment {
        service_id,
        commodity,
        proof: DeliveryAcknowledgmentProof,
        metadata: None,
        hashes: vec![],
    };
    socket.run(dack).await.unwrap();
    // Wait for aggregator to submit txn.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let total_served = query_runner
        .get_total_served(&query_runner.get_current_epoch())
        .expect("there to be total served information");
    assert_eq!(total_served.served[service_id as usize], commodity);

    node.shutdown().await;

    if path.exists() {
        std::fs::remove_file(path).unwrap();
    }
}
