use std::time::Duration;

use futures::future::join_all;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::prelude::*;
use lightning_test_utils::e2e::{
    DowncastToTestFullNode,
    TestFullNodeComponentsWithRealConsensus,
    TestNetwork,
};
use lightning_utils::application::QueryRunnerExt;
use lightning_utils::poll::{poll_until, PollUntilError};
use types::{
    ExecuteTransactionError,
    ExecuteTransactionOptions,
    ExecutionData,
    TransactionResponse,
    UpdateMethod,
};

#[tokio::test]
async fn test_insufficient_nodes_in_committee() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        // We need at least 2 nodes in the committee or else transactions will not execute.
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(1)
        .await
        .build()
        .await
        .unwrap();

    // Attempt to execute an increment nonce transaction from the node.
    let result = network
        .node(0)
        .execute_transaction_from_node(
            UpdateMethod::IncrementNonce {},
            Some(ExecuteTransactionOptions {
                // Transactions that are submitted immediately after startup will sometimes
                // timeout and need to be retried.
                wait: types::ExecuteTransactionWait::Receipt,
                retry: types::ExecuteTransactionRetry::Always(Some(3)),
                timeout: Some(Duration::from_secs(1)),
            }),
        )
        .await;
    match result.unwrap_err() {
        ExecuteTransactionError::FailedToIncrementNonceForRetry((_, msg)) => {
            assert_eq!(msg, "Timeout reached");
        },
        error => panic!(
            "expected FailedToIncrementNonceForRetry error, got {:?}",
            error
        ),
    }

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_execute_transaction_as_committee_node() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(4)
        .await
        .build()
        .await
        .unwrap();

    // Execute an increment nonce transaction from the first node.
    let (_, receipt) = network
        .node(0)
        .execute_transaction_from_node(
            UpdateMethod::IncrementNonce {},
            Some(ExecuteTransactionOptions {
                // Transactions that are submitted immediately after startup will sometimes
                // timeout and need to be retried.
                wait: types::ExecuteTransactionWait::Receipt,
                retry: types::ExecuteTransactionRetry::Always(Some(10)),
                timeout: Some(Duration::from_secs(2)),
            }),
        )
        .await
        .unwrap()
        .as_receipt();
    assert_eq!(
        receipt.response,
        TransactionResponse::Success(ExecutionData::None)
    );

    // Check that the node nonce was incremented across the network.
    poll_until(
        || async {
            network
                .nodes()
                .all(|node| {
                    let nonce = node
                        .app_query()
                        .get_node_info(&0, |node| node.nonce)
                        .unwrap();
                    // When transactions are submitted immediately after startup, they may fail to
                    // initially make it to the mempool, in which case it will timeout and be
                    // retried, with a backfill of the first nonce. So we need to check for a range
                    // of nonces (in case of retry).
                    nonce > 0 && nonce < 5
                })
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(10),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_execute_transaction_as_non_committee_node() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(4)
        .await
        .with_non_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(1)
        .await
        .build()
        .await
        .unwrap();

    // Execute an increment nonce transaction from the non-committee node.
    let non_committee_node = network.non_committee_nodes()[0];
    let (_, receipt) = non_committee_node
        .execute_transaction_from_node(
            UpdateMethod::IncrementNonce {},
            Some(ExecuteTransactionOptions {
                // Transactions that are submitted immediately after startup will sometimes
                // timeout and need to be retried.
                wait: types::ExecuteTransactionWait::Receipt,
                retry: types::ExecuteTransactionRetry::Always(Some(10)),
                timeout: Some(Duration::from_secs(2)),
            }),
        )
        .await
        .unwrap()
        .as_receipt();
    assert_eq!(
        receipt.response,
        TransactionResponse::Success(ExecutionData::None)
    );

    // Check that the node nonce was incremented across the network.
    poll_until(
        || async {
            network
                .nodes()
                .all(|node| {
                    let nonce = node
                        .app_query()
                        .get_node_info(&non_committee_node.index(), |node| node.nonce)
                        .unwrap();
                    // When transactions are submitted immediately after startup, they may fail to
                    // initially make it to the mempool, in which case it will timeout and be
                    // retried, with a backfill of the first nonce. So we need to check for a range
                    // of nonces (in case of retry).
                    nonce > 0 && nonce < 5
                })
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(3),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_epoch_change_via_time() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        .with_genesis_mutator(|genesis| {
            // Trigger epoch change on startup.
            genesis.epoch_start = 0;
        })
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(4)
        .await
        .build()
        .await
        .unwrap();

    // Check that the current epoch is 0 across the network.
    for node in network.nodes() {
        assert_eq!(node.app_query().get_current_epoch(), 0);
    }

    // Wait for epoch to be incremented across the network.
    poll_until(
        || async {
            network
                .nodes()
                .all(|node| node.app_query().get_current_epoch() == 1)
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(20),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_epoch_change_via_transactions() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(4)
        .await
        .build()
        .await
        .unwrap();

    // Check that the current epoch is 0 across the network.
    for node in network.nodes() {
        assert_eq!(node.app_query().get_current_epoch(), 0);
    }

    // Execute change epoch transactions from 2/3+1 of the committee nodes.
    join_all(network.nodes().take(3).map(|node| {
        node.execute_transaction_from_node(
            UpdateMethod::ChangeEpoch { epoch: 0 },
            Some(ExecuteTransactionOptions {
                // Transactions that are submitted immediately after startup will sometimes
                // timeout and need to be retried.
                wait: types::ExecuteTransactionWait::Receipt,
                retry: types::ExecuteTransactionRetry::Always(Some(5)),
                timeout: Some(Duration::from_secs(5)),
            }),
        )
    }))
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .unwrap();

    // Wait for epoch to be incremented across the network.
    poll_until(
        || async {
            network
                .nodes()
                .all(|node| node.app_query().get_current_epoch() == 1)
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(3),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Shutdown the network.
    network.shutdown().await;
}

#[tokio::test]
async fn test_node_has_insufficient_stake_to_participate_after_unstake_transaction() {
    let mut network = TestNetwork::builder()
        .with_real_consensus()
        .with_committee_nodes::<TestFullNodeComponentsWithRealConsensus>(4)
        .await
        .build()
        .await
        .unwrap();

    // Check the initial committee and active node set.
    for node in network.nodes() {
        let query = node.app_query();
        assert_eq!(
            query
                .get_committee_info(&0, |committee| committee.members)
                .unwrap(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            query
                .get_committee_info(&0, |committee| committee.active_node_set)
                .unwrap(),
            vec![0, 1, 2, 3]
        );
    }

    // Get the node's current stake.
    let node_stake = network
        .node(0)
        .app_query()
        .get_node_info(&0, |node| node.stake.staked)
        .unwrap();

    // Get the min stake.
    let min_stake: HpUfixed<18> = network.node(0).app_query().get_staking_amount().into();

    // Execute an unstake transaction to reduce the node's stake below the minimum.
    let (_, receipt) = network
        .node(0)
        .downcast::<TestFullNodeComponentsWithRealConsensus>()
        .execute_transaction_from_owner(
            UpdateMethod::Unstake {
                amount: node_stake - (min_stake.clone() - HpUfixed::<18>::from(1u64)),
                node: network.node(0).get_node_public_key(),
            },
            Some(ExecuteTransactionOptions {
                // Transactions that are submitted immediately after startup will sometimes
                // timeout and need to be retried.
                wait: types::ExecuteTransactionWait::Receipt,
                retry: types::ExecuteTransactionRetry::Always(Some(10)),
                timeout: Some(Duration::from_secs(2)),
            }),
        )
        .await
        .unwrap()
        .as_receipt();
    assert_eq!(
        receipt.response,
        TransactionResponse::Success(ExecutionData::None)
    );

    // Check that the node's stake was updated in app state across the network.
    poll_until(
        || async {
            network
                .nodes()
                .all(|node| {
                    let node_stake = node
                        .app_query()
                        .get_node_info(&0, |node| node.stake.staked)
                        .unwrap();
                    node_stake == min_stake.clone() - HpUfixed::<18>::from(1u64)
                })
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(3),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // Check that the committee members and active node set has NOT changed yet.
    for node in network.nodes() {
        let query = node.app_query();
        assert_eq!(
            query
                .get_committee_info(&0, |committee| committee.members)
                .unwrap(),
            vec![0, 1, 2, 3]
        );
        assert_eq!(
            query
                .get_committee_info(&0, |committee| committee.active_node_set)
                .unwrap(),
            vec![0, 1, 2, 3]
        );
    }

    // // Execute change epoch transactions to trigger epoch change.
    // join_all(
    //     network
    //         .nodes()
    //         .filter(|node| node.index() != 0)
    //         .map(|node| {
    //             node.execute_transaction_from_node(UpdateMethod::ChangeEpoch { epoch: 0 }, None)
    //         }),
    // )
    // .await
    // .into_iter()
    // .collect::<Result<Vec<_>, _>>()
    // .unwrap();

    // // Wait for epoch change to propagate across the network.
    // poll_until(
    //     || async {
    //         network
    //             .nodes()
    //             .all(|node| node.app_query().get_current_epoch() == 1)
    //             .then_some(())
    //             .ok_or(PollUntilError::ConditionNotSatisfied)
    //     },
    //     Duration::from_secs(3),
    //     Duration::from_millis(100),
    // )
    // .await
    // .unwrap();

    // // Check that the node with insufficient stake has been removed from the committee and active
    // // node set.
    // for node in network.nodes() {
    //     let query = node.app_query();
    //     assert_eq!(
    //         query
    //             .get_committee_info(&0, |committee| committee.members)
    //             .unwrap(),
    //         vec![1, 2, 3]
    //     );
    //     assert_eq!(
    //         query
    //             .get_committee_info(&0, |committee| committee.active_node_set)
    //             .unwrap(),
    //         vec![1, 2, 3]
    //     );
    // }

    // Shutdown the network.
    network.shutdown().await;
}
