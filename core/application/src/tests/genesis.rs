use fleek_crypto::SecretKey;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{ExecutionError, UpdateMethod, UpdatePayload, UpdateRequest};
use lightning_interfaces::ToDigest;
use tempfile::tempdir;

use super::macros::*;
use super::utils::*;

#[tokio::test]
async fn test_genesis_configuration() {
    let temp_dir = tempdir().unwrap();

    // Init application + get the query and update socket
    let (_, query_runner) = init_app(&temp_dir, None);
    // Get the genesis parameters plus the initial committee
    let genesis = test_genesis();
    let genesis_committee = genesis.node_info;
    // For every member of the genesis committee they should have an initial stake of the min stake
    // Query to make sure that holds true
    for node in genesis_committee {
        let balance = get_staked(&query_runner, &node.primary_public_key);
        assert_eq!(HpUfixed::<18>::from(genesis.min_stake), balance);
    }
}

#[tokio::test]
async fn test_invalid_chain_id() {
    let temp_dir = tempdir().unwrap();

    let chain_id = CHAIN_ID + 1;
    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    // Submit a OptIn transaction that will revert (InvalidChainID).

    // Regular Txn Execution
    let secret_key = &keystore[0].node_secret_key;
    let payload = UpdatePayload {
        sender: secret_key.to_pk().into(),
        nonce: 1,
        method: UpdateMethod::OptIn {},
        chain_id,
    };
    let digest = payload.to_digest();
    let signature = secret_key.sign(&digest);
    let update = UpdateRequest {
        signature: signature.into(),
        payload: payload.clone(),
    };
    expect_tx_revert!(update, &update_socket, ExecutionError::InvalidChainId);
}
