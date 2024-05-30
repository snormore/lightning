use std::collections::BTreeMap;
use std::time::Duration;

use fleek_crypto::{AccountOwnerSecretKey, SecretKey};
use lightning_application::app::Application;
use lightning_application::config::{Config as AppConfig, Mode, StorageConfig};
use lightning_application::genesis::{Genesis, GenesisNode};
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{NodePorts, UpdateMethod};
use lightning_notifier::Notifier;
use lightning_test_utils::consensus::{Config as ConsensusConfig, MockConsensus, MockForwarder};
use lightning_test_utils::json_config::JsonConfigProvider;
use lightning_test_utils::keys::EphemeralKeystore;

use crate::Signer;

partial!(TestBinding {
    ConfigProviderInterface = JsonConfigProvider;
    KeystoreInterface = EphemeralKeystore<Self>;
    SignerInterface = Signer<Self>;
    ApplicationInterface = Application<Self>;
    ConsensusInterface = MockConsensus<Self>;
    ForwarderInterface = MockForwarder<Self>;
    NotifierInterface = Notifier<Self>;
});

fn build_node(transactions_to_lose: &[u32]) -> Node<TestBinding> {
    let keystore = EphemeralKeystore::<TestBinding>::default();
    let (consensus_secret_key, node_secret_key) =
        (keystore.get_bls_sk(), keystore.get_ed25519_sk());

    let mut genesis = Genesis::load(None).unwrap();
    let node_public_key = node_secret_key.to_pk();
    let consensus_public_key = consensus_secret_key.to_pk();
    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner_public_key = owner_secret_key.to_pk();

    genesis.node_info.push(GenesisNode::new(
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
    ));

    Node::<TestBinding>::init_with_provider(
        fdi::Provider::default().with(keystore).with(
            JsonConfigProvider::default()
                .with::<Application<TestBinding>>(AppConfig {
                    genesis: Some(genesis),
                    genesis_path: None,
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
                    transactions_to_lose: transactions_to_lose.iter().copied().collect(),
                    new_block_interval: Duration::from_secs(5),
                }),
        ),
    )
    .expect("Failed to init node.")
}

fn get_our_nonce<C: Collection>(node: &Node<C>) -> u64 {
    let query_runner = node.provider.get::<C::ApplicationInterface>().sync_query();
    let node_public_key = node.provider.get::<C::KeystoreInterface>().get_ed25519_pk();
    let node_idx = query_runner.pubkey_to_index(&node_public_key).unwrap();
    query_runner
        .get_node_info::<u64>(&node_idx, |n| n.nonce)
        .unwrap()
}

#[tokio::test]
async fn test_send_two_txs_in_a_row() {
    let node = build_node(&[]);
    node.start().await;

    let signer_socket = node.provider.get::<Signer<TestBinding>>().get_socket();

    // Send two transactions to the signer.
    let update_method = UpdateMethod::SubmitReputationMeasurements {
        measurements: BTreeMap::new(),
    };
    signer_socket.run(update_method).await.unwrap();
    let update_method = UpdateMethod::SubmitReputationMeasurements {
        measurements: BTreeMap::new(),
    };
    signer_socket.run(update_method).await.unwrap();

    // Each transaction will take at most 2 seconds to get ordered.
    // Therefore, after 5 seconds, the nonce should be 2.
    tokio::time::sleep(Duration::from_secs(5)).await;
    let new_nonce = get_our_nonce(&node);
    assert_eq!(new_nonce, 2);
}

#[tokio::test]
async fn test_retry_send() {
    let node = build_node(&[2]);
    node.start().await;

    let signer_socket = node.provider.get::<Signer<TestBinding>>().get_socket();

    // Send two transactions to the signer. The OptIn transaction was chosen arbitrarily.
    let update_method = UpdateMethod::OptIn {};
    signer_socket.run(update_method).await.unwrap();
    // This transaction won't be ordered and the nonce won't be incremented on the application.
    let update_method = UpdateMethod::OptIn {};
    signer_socket.run(update_method).await.unwrap();
    // This transaction will have the wrong nonce, since the signer increments nonces
    // optimistically.
    let update_method = UpdateMethod::OptIn {};
    signer_socket.run(update_method).await.unwrap();

    // The signer will notice that the nonce doesn't increment on the application after the second
    // transaction, and then it will resend all following transactions.
    // Hence, the application nonce should be 3 after some time.
    tokio::time::sleep(Duration::from_secs(15)).await;
    let new_nonce = get_our_nonce(&node);
    assert_eq!(new_nonce, 3);
}
