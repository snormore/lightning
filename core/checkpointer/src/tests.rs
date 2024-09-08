use std::time::Duration;

use anyhow::Result;
use bit_set::BitSet;
use lightning_interfaces::types::{AggregateCheckpointHeader, CheckpointHeader};
use pretty_assertions::assert_eq;
use tempfile::tempdir;

use crate::test_utils::{TestNetworkBuilder, TestNodeBuilder, WaitUntilError};

#[tokio::test]
async fn test_start_shutdown() -> Result<()> {
    let temp_dir = tempdir()?;
    let _node = TestNodeBuilder::new(temp_dir.path().to_path_buf())
        .build()
        .await?;

    Ok(())
}

#[tokio::test]
async fn test_over_epoch_changes() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;

    for epoch in 0..10 {
        // Emit epoch changed notification to all nodes.
        network
            .notify_epoch_changed(epoch, [2; 32].into(), [3; 32].into(), [1; 32])
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
                        previous_state_root: [2; 32].into(),
                        next_state_root: [3; 32].into(),
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
                    state_root: [3; 32].into(),
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
async fn test_no_supermajority_of_attestations() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we have 2 nodes with the same next state root, and 1 node with a different next state
    // root. They all have the same epochs, serialized state digests, and previous state roots.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(2, epoch, [1; 32], [2; 32].into(), [11; 32].into())
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
async fn test_missing_epoch_change_notification_no_supermajority() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we emit epoch changed notifications to two of the nodes, and not the third.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32].into(), [10; 32].into())
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
async fn test_missing_epoch_change_notification_still_supermajority() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(4).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to all nodes, with different state roots so that there is no
    // supermajority.
    // Here we emit epoch changed notifications to three of the nodes, and not the fourth, so that
    // there is still a supermajority.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(2, epoch, [1; 32], [2; 32].into(), [10; 32].into())
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
                state_root: [10; 32].into(),
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

#[tokio::test]
async fn test_aggregate_checkpoint_header_already_exists() -> Result<()> {
    // TODO(snormore): Remove this tracing setup when finished debugging.
    crate::test_utils::init_tracing();

    let mut network = TestNetworkBuilder::new().with_num_nodes(1).build().await?;
    let epoch = 1001;

    // Emit epoch changed notifications.
    network
        .notify_epoch_changed(epoch, [3; 32].into(), [10; 32].into(), [1; 32])
        .await;

    // Get the stored checkpoint headers.
    let _headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers| headers.len() == 1)
        .await?;

    // Get the stored aggregate checkpoint header.
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    assert_eq!(agg_header_by_node.len(), 1);
    let expected_agg_header = AggregateCheckpointHeader {
        epoch,
        state_root: [10; 32].into(),
        nodes: BitSet::from_iter(vec![0]),
        signature: agg_header_by_node[&0].signature,
    };
    assert_eq!(agg_header_by_node[&0], expected_agg_header);

    // Emit the same epoch changed notification again, with a different state root so that
    // the resulting aggregate checkpoint header is different.
    network
        .notify_epoch_changed(epoch, [4; 32].into(), [11; 32].into(), [2; 32])
        .await;

    // Check that the node has not stored a new aggregate checkpoint header.
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    assert_eq!(agg_header_by_node.len(), 1);
    assert_eq!(agg_header_by_node[&0], expected_agg_header);

    // Shutdown the network.
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_delayed_epoch_change_notification() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch = 1001;

    // Emit epoch changed notification to 2 of 3 nodes.
    network
        .notify_node_epoch_changed(0, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;

    // Wait for 2 checkpoint headers to be stored in all 3 nodes.
    let _headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers_by_node| {
            headers_by_node.values().all(|headers| headers.len() == 2)
        })
        .await?;

    // Emit epoch changed notification to the third node.
    network
        .notify_node_epoch_changed(2, epoch, [1; 32], [2; 32].into(), [10; 32].into())
        .await;

    // Wait for the third node to receive the epoch changed notification, broadcast it's checkpoint
    // header, and for it to be stored in all the nodes.
    let _headers_by_node = network
        .wait_for_checkpoint_headers(epoch, |headers_by_node| {
            headers_by_node.values().all(|headers| headers.len() == 3)
        })
        .await?;

    // Check that the third node has constructed and stored the aggregate checkpoint header, in this
    // case the responsibility of the epoch change listener itself because nodes don't broadcast to
    // themselves.
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    assert_eq!(agg_header_by_node.len(), 3);
    assert_eq!(
        agg_header_by_node[&0],
        AggregateCheckpointHeader {
            epoch,
            state_root: [10; 32].into(),
            nodes: BitSet::from_iter(vec![0, 1, 2]),
            signature: agg_header_by_node[&0].signature,
        }
    );

    // Shutdown the network.
    network.shutdown().await;
    Ok(())
}

#[tokio::test]
async fn test_multiple_different_epochs_simultaneously() -> Result<()> {
    let mut network = TestNetworkBuilder::new().with_num_nodes(3).build().await?;
    let epoch1 = 1001;
    let epoch2 = 1002;

    // Emit epoch changed notifications to all nodes for both epochs, interleaved for each node.
    network
        .notify_node_epoch_changed(0, epoch1, [11; 32], [12; 32].into(), [110; 32].into())
        .await;
    network
        .notify_node_epoch_changed(0, epoch2, [21; 32], [22; 32].into(), [210; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch1, [11; 32], [12; 32].into(), [110; 32].into())
        .await;
    network
        .notify_node_epoch_changed(1, epoch2, [21; 32], [22; 32].into(), [210; 32].into())
        .await;
    network
        .notify_node_epoch_changed(2, epoch1, [11; 32], [12; 32].into(), [110; 32].into())
        .await;
    network
        .notify_node_epoch_changed(2, epoch2, [21; 32], [22; 32].into(), [210; 32].into())
        .await;

    // Check that the nodes have received and stored the checkpoint headers for both epochs.
    let epoch1_headers_by_node = network
        .wait_for_checkpoint_headers(epoch1, |headers_by_node| {
            headers_by_node.values().all(|headers| headers.len() == 3)
        })
        .await?;
    for (_node_id, headers) in epoch1_headers_by_node.iter() {
        assert_eq!(headers.len(), 3);
        for header in headers.iter() {
            assert!(network.verify_checkpointer_header_signature(header.clone())?);
            assert_eq!(
                header,
                &CheckpointHeader {
                    node_id: header.node_id,
                    epoch: epoch1,
                    previous_state_root: [12; 32].into(),
                    next_state_root: [110; 32].into(),
                    serialized_state_digest: [11; 32],
                    // The signature is verified separately.
                    signature: header.signature,
                }
            );
        }
    }
    let epoch2_headers_by_node = network
        .wait_for_checkpoint_headers(epoch2, |headers_by_node| {
            headers_by_node.values().all(|headers| headers.len() == 3)
        })
        .await?;
    for (_node_id, headers) in epoch2_headers_by_node.iter() {
        assert_eq!(headers.len(), 3);
        for header in headers.iter() {
            assert!(network.verify_checkpointer_header_signature(header.clone())?);
            assert_eq!(
                header,
                &CheckpointHeader {
                    node_id: header.node_id,
                    epoch: epoch2,
                    previous_state_root: [22; 32].into(),
                    next_state_root: [210; 32].into(),
                    serialized_state_digest: [21; 32],
                    // The signature is verified separately.
                    signature: header.signature,
                }
            );
        }
    }

    // Check that the nodes have constructed and stored the aggregate checkpoint headers for both
    // epochs.
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch1, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    for (node_id, agg_header) in agg_header_by_node.iter() {
        // Verify the aggregate header signature.
        assert!(network.verify_aggregate_checkpointer_header(
            agg_header.clone(),
            *node_id,
            epoch1_headers_by_node.clone(),
        )?);

        // Check that the aggregate header is correct.
        assert_eq!(
            agg_header,
            &AggregateCheckpointHeader {
                epoch: epoch1,
                state_root: [110; 32].into(),
                nodes: BitSet::from_iter(vec![0, 1, 2]),
                // The signature is verified separately.
                signature: agg_header.signature,
            }
        );
    }
    let agg_header_by_node = network
        .wait_for_aggregate_checkpoint_header(epoch2, |header_by_node| {
            header_by_node.values().all(|header| header.is_some())
        })
        .await?;
    for (node_id, agg_header) in agg_header_by_node.iter() {
        // Verify the aggregate header signature.
        assert!(network.verify_aggregate_checkpointer_header(
            agg_header.clone(),
            *node_id,
            epoch2_headers_by_node.clone(),
        )?);

        // Check that the aggregate header is correct.
        assert_eq!(
            agg_header,
            &AggregateCheckpointHeader {
                epoch: epoch2,
                state_root: [210; 32].into(),
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
// async fn test_attestation_with_invalid_signature() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_attestations_with_inconsistent_state_roots_no_supermajority() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_duplicate_epoch_change_notifications_on_same_epoch() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_different_epoch_change_notification_on_same_epoch() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_duplicate_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_too_few_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_missing_attestations() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_epoch_change_listener_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }

// #[tokio::test]
// async fn test_attestation_listener_panic_causes_shutdown() -> Result<()> {
//     // TODO(snormore): Implement this test.
//     Ok(())
// }
