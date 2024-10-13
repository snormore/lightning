use std::time::Duration;

use lightning_interfaces::types::{Metadata, UpdateMethod};
use lightning_interfaces::CommitteeBeaconQueryInterface;
use lightning_test_utils::e2e::{NetworkQueryRunner, TestNetworkBuilder, TestNodeBuilder};
use lightning_utils::poll::{poll_until, PollUntilError};
use tempfile::tempdir;

#[tokio::test]
async fn test_start_shutdown() {
    let temp_dir = tempdir().unwrap();
    let node = TestNodeBuilder::new(temp_dir.path().to_path_buf())
        .build()
        .await
        .unwrap();
    node.shutdown().await;
}

#[tokio::test]
async fn test_epoch_change_single_node() {
    let network = TestNetworkBuilder::new()
        .with_num_nodes(1)
        .build()
        .await
        .unwrap();
    let node = network.node(0);
    let query = node.application_query();

    // Send epoch change transaction from all nodes.
    let epoch = network.change_epoch().await;

    // Check that beacon phase is set.
    // We don't check for commit phase specifically because we can't be sure it hasn't transitioned
    // to the reveal phase before checking.
    poll_until(
        || async {
            query
                .get_committee_selection_beacon_phase()
                .is_some()
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(5),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Check that beacons are in app state.
    // These difficult to catch this at the right time with queries, so we just check that the
    // number is less than or equal to the number of nodes.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.len() <= network.node_count());

    // Check that beacons are in local database.
    // These difficult to catch this at the right time with queries, so we just check that the
    // number is less than or equal to the number of nodes.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.len() <= network.node_count());

    // Wait for reveal phase to complete and beacon phase to be unset.
    wait_for_committee_selection_beacon_phase_unset(&*query)
        .await
        .unwrap();

    // Check that the epoch has been incremented.
    let new_epoch = query.get_epoch();
    assert_eq!(new_epoch, epoch);

    // Check that there are no node beacons (commits and reveals) in app state.
    let beacons = query.get_committee_selection_beacons();
    assert!(beacons.is_empty());

    // Clearing the beacons at epoch change is best-effort, since we can't guarantee that
    // the notification will be received or the listener will be running, in the case of a
    // deployment for example. This is fine, since the beacons will be cleared on the next
    // committee selection phase anyway, and we don't rely on it for correctness.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.len() <= network.node_count());

    // TODO(snormore): Check that the next commmittee was selected.

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_epoch_change_multiple_nodes() {
    let network = TestNetworkBuilder::new()
        .with_num_nodes(3)
        .build()
        .await
        .unwrap();
    let node = network.node(0);
    let query = node.application_query();

    // Send epoch change transaction from all nodes.
    let epoch = network.change_epoch().await;

    // Check that beacon phase is set.
    // We don't check for commit phase specifically because we can't be sure it hasn't transitioned
    // to the reveal phase before checking.
    poll_until(
        || async {
            query
                .get_committee_selection_beacon_phase()
                .is_some()
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(5),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Check that beacons are in app state.
    // It's difficult to catch this at the right time with queries, so we just check that the
    // number is less than or equal to the number of nodes.
    let beacons = query.get_committee_selection_beacons();
    assert!(beacons.len() <= network.node_count());

    // Check that beacons are in local database.
    // It's difficult to catch this at the right time with queries, so we just check that the
    // number is less than or equal to the number of nodes.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.len() <= network.node_count());

    // Wait for reveal phase to complete and beacon phase to be unset.
    wait_for_committee_selection_beacon_phase_unset(&*query)
        .await
        .unwrap();

    // Check that the epoch has been incremented.
    let new_epoch = query.get_epoch();
    assert_eq!(new_epoch, epoch);

    // Check that there are no node beacons (commits and reveals) in app state.
    let beacons = query.get_committee_selection_beacons();
    assert!(beacons.is_empty());

    // Clearing the beacons at epoch change is best-effort, since we can't guarantee that
    // the notification will be received or the listener will be running, in the case of a
    // deployment for example. This is fine, since the beacons will be cleared on the next
    // committee selection phase anyway, and we don't rely on it for correctness.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.len() <= network.node_count());

    // TODO(snormore): Check that the next commmittee was selected.

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_block_executed_in_waiting_phase_should_do_nothing() {
    let network = TestNetworkBuilder::new()
        .with_num_nodes(2)
        .build()
        .await
        .unwrap();
    let node = network.node(0);
    let query = node.application_query();

    // Check beacon phase before submitting transaction.
    let phase = query.get_committee_selection_beacon_phase();
    assert!(phase.is_none());

    // Submit a transaction that does nothing except increment the node's nonce.
    node.node_transaction_client()
        .await
        .execute_transaction_and_wait_for_receipt(UpdateMethod::IncrementNonce {}, None)
        .await
        .unwrap();

    // Check that beacon phase has not changed.
    let phase = query.get_committee_selection_beacon_phase();
    assert!(phase.is_none());

    // Check that there are no node beacons (commits and reveals) in app state.
    let beacons = query.get_committee_selection_beacons();
    assert!(beacons.is_empty());

    // Check that there are no beacons in our local database.
    let beacons = node.committee_beacon_query().get_beacons();
    assert!(beacons.is_empty());

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_insufficient_participation_in_commit_phase() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_insufficient_participation_in_reveal_phase() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_fails_to_reveal_after_committing() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_attempts_reveal_without_committment() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_invalid_reveal_mismatch_with_commit() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_submits_commit_outside_of_commit_phase() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_submits_reveal_outside_of_reveal_phase() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_reuses_old_commitment() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_reuses_old_reveal() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_non_committee_node_participation() {
    // TODO(snormore): Implement this test.

    // TODO(snormore): Check that the next commmittee was selected.
}

#[tokio::test]
async fn test_malformed_commit() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_malformed_reveal() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_non_revealing_node_attempts_to_commit_in_next_round() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_high_volume_participation() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_network_delays() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_node_attempts_to_submit_reveal_during_commit_phase() {
    // TODO(snormore): Implement this test.
}

#[tokio::test]
async fn test_multiple_non_revealing_nodes() {
    // TODO(snormore): Implement this test.
}

pub async fn wait_for_committee_selection_beacon_phase_unset(
    query: &dyn NetworkQueryRunner,
) -> Result<(), PollUntilError> {
    poll_until(
        || async {
            query
                .get_metadata(&Metadata::CommitteeSelectionBeaconPhase)
                .is_none()
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(30),
        Duration::from_millis(100),
    )
    .await
}
