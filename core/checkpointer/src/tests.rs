use std::time::Duration;

use anyhow::Result;
use bit_set::BitSet;
use lightning_interfaces::types::{AggregateCheckpointHeader, CheckpointHeader};
use pretty_assertions::assert_eq;
use tempfile::tempdir;

use crate::test_utils::{TestNetworkBuilder, TestNodeBuilder, WaitUntilError};

#[tokio::test]
async fn test_checkpointer_start_shutdown() -> Result<()> {
    let temp_dir = tempdir()?;
    let _node = TestNodeBuilder::new(temp_dir.path().to_path_buf())
        .build()
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_checkpointer_over_epoch_changes() -> Result<()> {
    // TODO(snormore): Remove this tracing setup when finished debugging.
    crate::test_utils::init_tracing();

    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;

    for epoch in 0..1 {
        // Emit epoch changed notification to all nodes.
        network
            .notify_epoch_changed(epoch, [2; 32], [3; 32], [1; 32])
            .await;

        // Check that the nodes have received and stored the checkpoint headers.
        let headers_by_node = network
            .wait_for_checkpoint_headers(epoch, |headers_by_node| {
                headers_by_node
                    .values()
                    .map(|headers| headers.len())
                    .collect::<Vec<_>>()
                    == vec![3, 3, 3]
            })
            .await?;
        for (_node_id, headers) in headers_by_node.iter() {
            assert_eq!(headers.len(), 3);
            for header in headers.iter() {
                assert!(network.verify_checkpointer_header_signature(header.clone())?);
                assert_eq!(
                    header,
                    &CheckpointHeader {
                        node_id: header.node_id,
                        epoch,
                        previous_state_root: [2; 32],
                        next_state_root: [3; 32],
                        serialized_state_digest: [1; 32],
                        // The signature is verified separately.
                        signature: header.signature,
                    }
                );
            }
        }

        // Check that the nodes have constructed and stored the aggregate checkpoint header.
        let agg_header_by_node = network
            .wait_for_aggregate_checkpoint_header(epoch, |header_by_node| {
                header_by_node.values().all(|header| header.is_some())
            })
            .await?;
        for (node_id, agg_header) in agg_header_by_node.iter() {
            // Verify the aggregate header signature.
            assert!(network.verify_aggregate_checkpointer_header(
                agg_header.clone(),
                *node_id,
                headers_by_node.clone(),
            )?);

            // Check that the aggregate header is correct.
            assert_eq!(
                agg_header,
                &AggregateCheckpointHeader {
                    epoch,
                    previous_state_root: [2; 32],
                    next_state_root: [3; 32],
                    nodes: BitSet::from_iter(vec![0, 1, 2]),
                    // The signature is verified separately.
                    signature: agg_header.signature,
                }
            );
        }
    }

    // Shut down the network.
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_no_supermajority_of_attestations() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we have 2 nodes with the same next state root, and 1 node with a different next state
    // root. They all have the same epochs, serialized state digests, and previous state roots.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32], [10; 32])
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32], [10; 32])
        .await;
    network
        .notify_node_epoch_changed(2, epoch, [1; 32], [2; 32], [11; 32])
        .await;

    // Check that the nodes have received and stored the checkpoint headers.
    let headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers_by_node| {
            headers_by_node
                .values()
                .map(|headers| headers.len())
                .collect::<Vec<_>>()
                == vec![3, 3, 3]
        })
        .await?;
    for (_node_id, headers) in headers_by_node.iter() {
        assert_eq!(headers.len(), 3);
        for header in headers.iter() {
            assert!(network.verify_checkpointer_header_signature(header.clone())?);
        }
    }

    // Check that the nodes have not stored an aggregate checkpoint header, because there is no
    // supermajority.
    // TODO(snormore): Consider adding a EpochCheckpointNotification type to the notifier and using
    // the non-receipt of it as our check here.
    let result = network
        .wait_for_aggregate_checkpoint_header_with_timeout(
            epoch,
            |header_by_node| header_by_node.values().all(|header| header.is_some()),
            Duration::from_secs(1),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), WaitUntilError::Timeout);

    // Shutdown the network.
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_missing_epoch_change_notification_no_supermajority() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we emit epoch changed notifications to two of the nodes, and not the third.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32], [10; 32])
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32], [10; 32])
        .await;

    // Check that the nodes have received and stored the checkpoint headers.
    // Note that we only get 2 headers per node, because one of the nodes did not receive an epoch
    // changed notification.
    let headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers_by_node| {
            headers_by_node
                .values()
                .map(|headers| headers.len())
                .collect::<Vec<_>>()
                == vec![2, 2, 2]
        })
        .await?;
    for (_node_id, headers) in headers_by_node.iter() {
        assert_eq!(headers.len(), 2);
        for header in headers.iter() {
            assert!(network.verify_checkpointer_header_signature(header.clone())?);
        }
    }

    // Check that the nodes have not stored an aggregate checkpoint header, because there is no
    // supermajority.
    // TODO(snormore): Consider adding a EpochCheckpointNotification type to the notifier and using
    // the non-receipt of it as our check here.
    let result = network
        .wait_for_aggregate_checkpoint_header_with_timeout(
            epoch,
            |header_by_node| header_by_node.values().all(|header| header.is_some()),
            Duration::from_secs(1),
        )
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), WaitUntilError::Timeout);

    // Shutdown the network.
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_checkpointer_missing_epoch_change_notification_still_supermajority() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(4).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we emit epoch changed notifications to three of the nodes, and not the fourth, so that
    // there is still a supermajority.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32], [10; 32])
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32], [10; 32])
        .await;
    network
        .notify_node_epoch_changed(2, epoch, [1; 32], [2; 32], [10; 32])
        .await;

    // Check that the nodes have received and stored the checkpoint headers.
    // Note that we only get 2 headers per node, because one of the nodes did not receive an epoch
    // changed notification.
    let headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers_by_node| {
            headers_by_node
                .values()
                .map(|headers| headers.len())
                .collect::<Vec<_>>()
                == vec![3, 3, 3, 3]
        })
        .await?;
    for (_node_id, headers) in headers_by_node.iter() {
        assert_eq!(headers.len(), 3);
        for header in headers.iter() {
            assert!(network.verify_checkpointer_header_signature(header.clone())?);
        }
    }

    // Check that the nodes have constructed and stored the aggregate checkpoint header.
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    for (node_id, agg_header) in agg_header_by_node.iter() {
        // Verify the aggregate header signature.
        assert!(network.verify_aggregate_checkpointer_header(
            agg_header.clone(),
            *node_id,
            headers_by_node.clone(),
        )?);

        // Check that the aggregate header is correct.
        assert_eq!(
            agg_header,
            &AggregateCheckpointHeader {
                epoch,
                previous_state_root: [2; 32],
                next_state_root: [10; 32],
                nodes: BitSet::from_iter(vec![0, 1, 2]),
                // The signature is verified separately.
                signature: agg_header.signature,
            }
        );
    }

    // Shutdown the network.
    network.shutdown().await;
    Ok(())
}

// #[tokio::test]
// async fn test_checkpointer_different_epochs() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_fake_and_corrupt_attestation() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_duplicate_epoch_change_notifications_on_same_epoch() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_different_epoch_change_notification_on_same_epoch() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_duplicate_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_too_few_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_missing_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_epoch_change_listener_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_checkpointer_attestation_listener_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }
