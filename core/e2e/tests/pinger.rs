use std::fs;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use lightning_e2e::swarm::Swarm;
use lightning_interfaces::types::Participation;
use lightning_rpc::api::RpcClient;
use lightning_rpc::interface::Fleek;
use lightning_test_utils::config::LIGHTNING_TEST_HOME_DIR;
use lightning_test_utils::logging;
use resolved_pathbuf::ResolvedPathBuf;
use serial_test::serial;

#[tokio::test]
#[serial]
async fn e2e_detect_offline_node() -> Result<()> {
    logging::setup();

    // Start epoch now and let it end in 40 seconds.
    let epoch_start = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let path = ResolvedPathBuf::try_from(LIGHTNING_TEST_HOME_DIR.join("e2e/pinger")).unwrap();
    if path.exists() {
        fs::remove_dir_all(&path).expect("Failed to clean up swarm directory before test.");
    }
    let swarm = Swarm::builder()
        .with_directory(path)
        .with_min_port(10500)
        .with_num_nodes(5)
        .with_committee_size(4)
        .with_epoch_time(25000)
        .with_epoch_start(epoch_start)
        .persistence(true)
        .build();
    swarm.launch_genesis_committee().await.unwrap();

    // Wait for the epoch to change.
    tokio::time::sleep(Duration::from_secs(30)).await;

    for (_, address) in swarm.get_genesis_committee_rpc_addresses() {
        let client = RpcClient::new_no_auth(&address)?;
        let epoch = client.get_epoch().await?;

        assert_eq!(epoch, 1);
    }

    // Get the public key of the node that was offline.
    let (pubkey, _) = swarm
        .get_non_genesis_committee_rpc_addresses()
        .into_iter()
        .next()
        .unwrap();

    // Make sure that the offline node was removed from participation.
    for (_, address) in swarm.get_genesis_committee_rpc_addresses() {
        let client = RpcClient::new_no_auth(&address)?;
        let node_info = client
            .get_node_info(pubkey, None)
            .await?
            .expect("No node info recieved from rpc");

        assert_eq!(node_info.participation, Participation::False);
    }

    swarm.shutdown().await;
    Ok(())
}
