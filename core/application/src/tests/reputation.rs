use std::collections::BTreeMap;

use fleek_crypto::{AccountOwnerSecretKey, NodeSecretKey, SecretKey};
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    ContentUpdate,
    ExecutionError,
    Participation,
    UpdateMethod,
    MAX_MEASUREMENTS_PER_TX,
    MAX_MEASUREMENTS_SUBMIT,
};
use lightning_interfaces::SyncQueryRunnerInterface;
use lightning_test_utils::{random, reputation};
use lightning_utils::application::QueryRunnerExt;
use tempfile::tempdir;

use super::macros::*;
use super::utils::*;

#[tokio::test]
async fn test_submit_rep_measurements() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let mut rng = random::get_seedable_rng();

    let mut map = BTreeMap::new();
    let update1 = update_reputation_measurements(
        &query_runner,
        &mut map,
        &keystore[1].node_secret_key.to_pk(),
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );
    let update2 = update_reputation_measurements(
        &query_runner,
        &mut map,
        &keystore[2].node_secret_key.to_pk(),
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );

    let reporting_node_key = keystore[0].node_secret_key.to_pk();
    let reporting_node_index = get_node_index(&query_runner, &reporting_node_key);

    submit_reputation_measurements!(&update_socket, &keystore[0].node_secret_key, 1, map);

    assert_rep_measurements_update!(&query_runner, update1, reporting_node_index);
    assert_rep_measurements_update!(&query_runner, update2, reporting_node_index);
}

#[tokio::test]
async fn test_submit_rep_measurements_too_many_times() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let mut rng = random::get_seedable_rng();

    let mut map = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &keystore[1].node_secret_key.to_pk(),
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );

    // Attempt to submit reputation measurements 1 more time than allowed per epoch.
    // This transaction should revert because each node only can submit its reputation measurements
    // `MAX_MEASUREMENTS_SUBMIT` times.
    for i in 0..MAX_MEASUREMENTS_SUBMIT {
        let req = prepare_update_request_node(
            UpdateMethod::SubmitReputationMeasurements {
                measurements: map.clone(),
            },
            &keystore[0].node_secret_key,
            1 + i as u64,
        );
        expect_tx_success!(req, &update_socket);
    }
    let req = prepare_update_request_node(
        UpdateMethod::SubmitReputationMeasurements { measurements: map },
        &keystore[0].node_secret_key,
        1 + MAX_MEASUREMENTS_SUBMIT as u64,
    );
    expect_tx_revert!(
        req,
        &update_socket,
        ExecutionError::SubmittedTooManyTransactions
    );
}

#[tokio::test]
async fn test_rep_scores() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let required_signals = calculate_required_signals(committee_size);

    let mut rng = random::get_seedable_rng();

    let peer1 = keystore[2].node_secret_key.to_pk();
    let peer2 = keystore[3].node_secret_key.to_pk();
    let nonce = 1;

    let mut map = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer1,
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer2,
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );
    submit_reputation_measurements!(&update_socket, &keystore[0].node_secret_key, nonce, map);

    let mut map = BTreeMap::new();
    let (peer_idx_1, _) = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer1,
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );
    let (peer_idx_2, _) = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer2,
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );
    submit_reputation_measurements!(&update_socket, &keystore[1].node_secret_key, nonce, map);

    let epoch = 0;
    // Change epoch so that rep scores will be calculated from the measurements.
    for (i, node) in keystore.iter().enumerate().take(required_signals) {
        // Not the prettiest solution but we have to keep track of the nonces somehow.
        let nonce = if i < 2 { 2 } else { 1 };
        change_epoch!(&update_socket, &node.node_secret_key, nonce, epoch);
    }

    assert!(query_runner.get_reputation_score(&peer_idx_1).is_some());
    assert!(query_runner.get_reputation_score(&peer_idx_2).is_some());
}

#[tokio::test]
async fn test_uptime_participation() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (mut committee, keystore) = create_genesis_committee(committee_size);
    committee[0].reputation = Some(40);
    committee[1].reputation = Some(80);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let required_signals = calculate_required_signals(committee_size);

    let peer_1 = keystore[2].node_secret_key.to_pk();
    let peer_2 = keystore[3].node_secret_key.to_pk();
    let nonce = 1;

    // Add records in the content registry for all nodes.
    let updates = vec![ContentUpdate {
        uri: [0u8; 32],
        remove: false,
    }];
    let content_registry_update =
        prepare_content_registry_update(updates.clone(), &keystore[2].node_secret_key, 1);
    expect_tx_success!(content_registry_update, &update_socket);
    let content_registry_update =
        prepare_content_registry_update(updates, &keystore[3].node_secret_key, 1);
    expect_tx_success!(content_registry_update, &update_socket);

    // Assert that registries have been updated.
    let index_peer1 = query_runner.pubkey_to_index(&peer_1).unwrap();
    let content_registry1 = content_registry(&query_runner, &index_peer1);
    assert!(!content_registry1.is_empty());

    let index_peer2 = query_runner.pubkey_to_index(&peer_2).unwrap();
    let content_registry2 = content_registry(&query_runner, &index_peer2);
    assert!(!content_registry2.is_empty());

    let providers = uri_to_providers(&query_runner, &[0u8; 32]);
    assert_eq!(providers.len(), 2);

    let mut map = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer_1,
        test_reputation_measurements(20),
    );
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer_2,
        test_reputation_measurements(40),
    );

    submit_reputation_measurements!(&update_socket, &keystore[0].node_secret_key, nonce, map);

    let mut map = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer_1,
        test_reputation_measurements(30),
    );

    let _ = update_reputation_measurements(
        &query_runner,
        &mut map,
        &peer_2,
        test_reputation_measurements(45),
    );
    submit_reputation_measurements!(&update_socket, &keystore[1].node_secret_key, nonce, map);

    let epoch = 0;
    // Change epoch so that rep scores will be calculated from the measurements.
    for node in keystore.iter().take(required_signals) {
        change_epoch!(&update_socket, &node.node_secret_key, 2, epoch);
    }

    let node_info1 = get_node_info(&query_runner, &peer_1);
    let node_info2 = get_node_info(&query_runner, &peer_2);

    assert_eq!(node_info1.participation, Participation::False);
    assert_eq!(node_info2.participation, Participation::True);

    // Assert that registries have been updated.
    let content_registry1 = content_registry(&query_runner, &index_peer1);
    assert!(content_registry1.is_empty());

    let content_registry2 = content_registry(&query_runner, &index_peer2);
    assert!(!content_registry2.is_empty());

    let providers = uri_to_providers(&query_runner, &[0u8; 32]);
    assert_eq!(providers.len(), 1);
}

#[tokio::test]
async fn test_submit_reputation_measurements_reverts_account_key() {
    let temp_dir = tempdir().unwrap();

    // Create a genesis committee and seed the application state with it.
    let committee_size = 4;
    let (committee, _keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    // Account Secret Key
    let secret_key = AccountOwnerSecretKey::generate();
    let opt_in = UpdateMethod::SubmitReputationMeasurements {
        measurements: Default::default(),
    };
    let update = prepare_update_request_account(opt_in, &secret_key, 1);
    expect_tx_revert!(update, &update_socket, ExecutionError::OnlyNode);
}

#[tokio::test]
async fn test_submit_reputation_measurements_reverts_node_does_not_exist() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let mut rng = random::get_seedable_rng();

    let mut measurements = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut measurements,
        &keystore[1].node_secret_key.to_pk(),
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );

    let update = prepare_update_request_node(
        UpdateMethod::SubmitReputationMeasurements { measurements },
        &NodeSecretKey::generate(),
        1,
    );

    expect_tx_revert!(update, &update_socket, ExecutionError::NodeDoesNotExist);
}

#[tokio::test]
async fn test_submit_reputation_measurements_reverts_insufficient_stake() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let mut rng = random::get_seedable_rng();

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_secret_key = NodeSecretKey::generate();

    // Stake less than the minimum required amount.
    let minimum_stake_amount: HpUfixed<18> = query_runner.get_staking_amount().into();
    let less_than_minimum_stake_amount: HpUfixed<18> =
        minimum_stake_amount / HpUfixed::<18>::from(2u16);
    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &less_than_minimum_stake_amount,
        &node_secret_key.to_pk(),
        [0; 96].into()
    );

    let mut measurements = BTreeMap::new();
    let _ = update_reputation_measurements(
        &query_runner,
        &mut measurements,
        &keystore[1].node_secret_key.to_pk(),
        reputation::generate_reputation_measurements(&mut rng, 0.1),
    );

    let update = prepare_update_request_node(
        UpdateMethod::SubmitReputationMeasurements { measurements },
        &node_secret_key,
        1,
    );

    expect_tx_revert!(update, &update_socket, ExecutionError::InsufficientStake);
}

#[tokio::test]
async fn test_submit_reputation_measurements_too_many_measurements() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, _keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let mut rng = random::get_seedable_rng();

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_secret_key = NodeSecretKey::generate();

    // Stake minimum required amount.
    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &query_runner.get_staking_amount().into(),
        &node_secret_key.to_pk(),
        [0; 96].into()
    );

    let mut measurements = BTreeMap::new();

    // create many dummy measurements that len >
    for i in 1..MAX_MEASUREMENTS_PER_TX + 2 {
        measurements.insert(
            i as u32,
            reputation::generate_reputation_measurements(&mut rng, 0.5),
        );
    }
    let update = prepare_update_request_node(
        UpdateMethod::SubmitReputationMeasurements { measurements },
        &node_secret_key,
        1,
    );

    expect_tx_revert!(update, &update_socket, ExecutionError::TooManyMeasurements);
}
