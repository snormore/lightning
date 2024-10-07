use std::time::{Duration, SystemTime};

use lightning_e2e::swarm::Swarm;
use lightning_rpc::interface::Fleek;
use lightning_rpc::RpcClient;
use lightning_test_utils::logging;
use lightning_utils::poll::{poll_until, PollUntilError};
use tempfile::tempdir;

#[tokio::test]
async fn e2e_checkpoint() {
    logging::setup();

    let temp_dir = tempdir().unwrap();
    let mut swarm = Swarm::builder()
        .with_directory(temp_dir.path().to_path_buf().try_into().unwrap())
        .with_min_port(10000)
        .with_num_nodes(4)
        // We need to include enough time in this epoch time for the nodes to start up, or else it
        // begins the epoch change immediately when they do, and potentially in a way that's out of
        // sync. We can even get into a situation where the next epoch change starts quickly after
        // that, and cause our check of epoch = 1 to fail.
        .with_epoch_time(15000)
        .with_epoch_start(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        )
        .persistence(true)
        .build();
    swarm.launch().await.unwrap();

    // Wait for RPC to be ready.
    swarm.wait_for_rpc_ready().await;

    // Wait for the epoch to change.
    swarm
        .wait_for_epoch_change(1, Duration::from_secs(60))
        .await
        .unwrap();

    // Wait until the last epoch hash is not all zeroes and equal across all nodes.
    poll_until(
        || async {
            // Check last epoch hash across all nodes.
            let mut target_hash = None;
            for (_, address) in swarm.get_rpc_addresses() {
                let client = RpcClient::new_no_auth(&address).unwrap();
                let (epoch_hash, _) = client.get_last_epoch_hash().await.unwrap();
                if target_hash.is_none() {
                    target_hash = Some(epoch_hash);
                }
                if epoch_hash != target_hash.unwrap() {
                    return Err(PollUntilError::ConditionNotSatisfied);
                }
            }

            // Check that the epoch hash is not all zeros, which would indicate that the checkpoint
            // was not stored, since that's the default.
            (target_hash.unwrap() != [0; 32])
                .then_some(())
                .ok_or(PollUntilError::ConditionNotSatisfied)
        },
        Duration::from_secs(5),
        Duration::from_millis(100),
    )
    .await
    .unwrap();

    // TODO(snormore): Read the block stores of all the nodes and make sure they all stored the
    // checkpoint.

    swarm.shutdown().await;
}
