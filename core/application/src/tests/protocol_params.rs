use fleek_crypto::{AccountOwnerSecretKey, SecretKey};
use lightning_interfaces::types::{ExecutionError, ProtocolParams, UpdateMethod};
use lightning_interfaces::SyncQueryRunnerInterface;
use lightning_utils::application::QueryRunnerExt;
use tempfile::tempdir;

use super::macros::*;
use super::utils::*;

#[tokio::test]
async fn test_change_protocol_params() {
    let temp_dir = tempdir().unwrap();

    let governance_secret_key = AccountOwnerSecretKey::generate();
    let governance_public_key = governance_secret_key.to_pk();

    let mut genesis = test_genesis();
    genesis.governance_address = governance_public_key.into();

    let (update_socket, query_runner) = init_app_with_genesis(&temp_dir, &genesis);

    let param = ProtocolParams::LockTime;
    let new_value = 5;
    let update =
        prepare_change_protocol_param_request(&param, &new_value, &governance_secret_key, 1);
    run_update!(update, &update_socket);
    assert_eq!(query_runner.get_protocol_param(&param).unwrap(), new_value);

    let new_value = 8;
    let update =
        prepare_change_protocol_param_request(&param, &new_value, &governance_secret_key, 2);
    run_update!(update, &update_socket);
    assert_eq!(query_runner.get_protocol_param(&param).unwrap(), new_value);

    // Make sure that another private key cannot change protocol parameters.
    let some_secret_key = AccountOwnerSecretKey::generate();
    let minimum_stake_amount = query_runner.get_staking_amount().into();
    deposit!(&update_socket, &some_secret_key, 1, &minimum_stake_amount);

    let malicious_value = 1;
    let update =
        prepare_change_protocol_param_request(&param, &malicious_value, &some_secret_key, 2);
    expect_tx_revert!(update, &update_socket, ExecutionError::OnlyGovernance);
    // Lock time should still be 8.
    assert_eq!(query_runner.get_protocol_param(&param).unwrap(), new_value)
}

#[tokio::test]
async fn test_change_protocol_params_reverts_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let param = ProtocolParams::LockTime;
    let initial_value = query_runner.get_protocol_param(&param).unwrap();
    let new_value = initial_value + 1;

    let change_method = UpdateMethod::ChangeProtocolParam {
        param: param.clone(),
        value: new_value,
    };

    // Assert that reverts for Node Key
    let update =
        prepare_update_request_node(change_method.clone(), &keystore[0].node_secret_key, 1);
    expect_tx_revert!(update, &update_socket, ExecutionError::OnlyAccountOwner);
    assert_eq!(
        query_runner.get_protocol_param(&param).unwrap(),
        initial_value
    );

    // Assert that reverts for Consensus Key
    let update = prepare_update_request_consensus(
        change_method.clone(),
        &keystore[0].consensus_secret_key,
        2,
    );
    expect_tx_revert!(update, &update_socket, ExecutionError::OnlyAccountOwner);
    assert_eq!(
        query_runner.get_protocol_param(&param).unwrap(),
        initial_value
    );
}
