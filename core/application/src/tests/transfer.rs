use fleek_crypto::{AccountOwnerSecretKey, EthAddress, NodeSecretKey, SecretKey};
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{ExecutionError, ProofOfConsensus, Tokens, UpdateMethod};
use lightning_interfaces::SyncQueryRunnerInterface;
use tempfile::tempdir;

use super::macros::*;
use super::utils::*;

#[tokio::test]
async fn test_revert_self_transfer() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner: EthAddress = owner_secret_key.to_pk().into();

    let balance = 1_000u64.into();

    deposit!(&update_socket, &owner_secret_key, 1, &balance);
    assert_eq!(get_flk_balance(&query_runner, &owner), balance);

    // Check that trying to transfer funds to yourself reverts
    let update = prepare_transfer_request(&10_u64.into(), &owner, &owner_secret_key, 2);
    expect_tx_revert!(update, &update_socket, ExecutionError::CantSendToYourself);

    // Assure that Flk balance has not changed
    assert_eq!(get_flk_balance(&query_runner, &owner), balance);
}

#[tokio::test]
async fn test_revert_transfer_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);
    let recipient: EthAddress = AccountOwnerSecretKey::generate().to_pk().into();

    let amount: HpUfixed<18> = 10_u64.into();
    let zero_balance = 0u64.into();

    assert_eq!(get_flk_balance(&query_runner, &recipient), zero_balance);

    let transfer = UpdateMethod::Transfer {
        amount: amount.clone(),
        token: Tokens::FLK,
        to: recipient,
    };

    // Check that trying to transfer funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key = prepare_update_request_node(transfer.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to transfer funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key = prepare_update_request_consensus(transfer, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Assure that Flk balance has not changed
    assert_eq!(get_flk_balance(&query_runner, &recipient), zero_balance);
}

#[tokio::test]
async fn test_revert_transfer_when_insufficient_balance() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let recipient: EthAddress = AccountOwnerSecretKey::generate().to_pk().into();

    let balance = 10_u64.into();
    let zero_balance = 0u64.into();

    deposit!(&update_socket, &owner_secret_key, 1, &balance);
    assert_eq!(get_flk_balance(&query_runner, &recipient), zero_balance);

    // Check that trying to transfer insufficient funds reverts
    let update = prepare_transfer_request(&11u64.into(), &recipient, &owner_secret_key, 2);
    expect_tx_revert!(update, &update_socket, ExecutionError::InsufficientBalance);

    // Assure that Flk balance has not changed
    assert_eq!(get_flk_balance(&query_runner, &recipient), zero_balance);
}

#[tokio::test]
async fn test_transfer_works_properly() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner: EthAddress = owner_secret_key.to_pk().into();
    let recipient: EthAddress = AccountOwnerSecretKey::generate().to_pk().into();

    let balance = 1_000u64.into();
    let zero_balance = 0u64.into();
    let transfer_amount: HpUfixed<18> = 10_u64.into();

    deposit!(&update_socket, &owner_secret_key, 1, &balance);

    assert_eq!(get_flk_balance(&query_runner, &owner), balance);
    assert_eq!(get_flk_balance(&query_runner, &recipient), zero_balance);

    // Check that trying to transfer funds to yourself reverts
    let update = prepare_transfer_request(&10_u64.into(), &recipient, &owner_secret_key, 2);
    expect_tx_success!(update, &update_socket);

    // Assure that Flk balance has decreased for sender
    assert_eq!(
        get_flk_balance(&query_runner, &owner),
        balance - transfer_amount.clone()
    );
    // Assure that Flk balance has increased for recipient
    assert_eq!(
        get_flk_balance(&query_runner, &recipient),
        zero_balance + transfer_amount
    );
}

#[tokio::test]
async fn test_deposit_flk_works_properly() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner: EthAddress = owner_secret_key.to_pk().into();

    let deposit_amount: HpUfixed<18> = 1_000u64.into();
    let intial_balance = get_flk_balance(&query_runner, &owner);

    let deposit = UpdateMethod::Deposit {
        proof: ProofOfConsensus {},
        token: Tokens::FLK,
        amount: deposit_amount.clone(),
    };
    let update = prepare_update_request_account(deposit, &owner_secret_key, 1);
    expect_tx_success!(update, &update_socket);

    assert_eq!(
        get_flk_balance(&query_runner, &owner),
        intial_balance + deposit_amount
    );
}

#[tokio::test]
async fn test_revert_deposit_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    let amount: HpUfixed<18> = 10_u64.into();
    let deposit = UpdateMethod::Deposit {
        proof: ProofOfConsensus {},
        token: Tokens::FLK,
        amount,
    };

    // Check that trying to deposit funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key = prepare_update_request_node(deposit.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to deposit funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key = prepare_update_request_consensus(deposit, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );
}

#[tokio::test]
async fn test_deposit_usdc_works_properly() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner: EthAddress = owner_secret_key.to_pk().into();

    let intial_balance = get_account_balance(&query_runner, &owner);
    let deposit_amount = 1_000;
    let deposit = UpdateMethod::Deposit {
        proof: ProofOfConsensus {},
        token: Tokens::USDC,
        amount: deposit_amount.into(),
    };
    let update = prepare_update_request_account(deposit, &owner_secret_key, 1);
    expect_tx_success!(update, &update_socket);

    assert_eq!(
        get_account_balance(&query_runner, &owner),
        intial_balance + deposit_amount
    );
}

#[tokio::test]
async fn test_withdraw_unstaked_reverts_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    let withdraw_unstaked = UpdateMethod::WithdrawUnstaked {
        node: NodeSecretKey::generate().to_pk(),
        recipient: None,
    };

    // Check that trying to Stake funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key =
        prepare_update_request_node(withdraw_unstaked.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to Stake funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key =
        prepare_update_request_consensus(withdraw_unstaked, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );
}

#[tokio::test]
async fn test_withdraw_unstaked_reverts_node_does_not_exist() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, _query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let update = prepare_withdraw_unstaked_update(&node_pub_key, None, &owner_secret_key, 1);

    expect_tx_revert!(update, &update_socket, ExecutionError::NodeDoesNotExist);
}

#[tokio::test]
async fn test_withdraw_unstaked_reverts_not_node_owner() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let amount: HpUfixed<18> = 1_000u64.into();

    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &amount,
        &node_pub_key,
        [0; 96].into()
    );

    assert_eq!(get_staked(&query_runner, &node_pub_key), amount);

    let withdraw_unstaked = prepare_withdraw_unstaked_update(
        &node_pub_key,
        None,
        &AccountOwnerSecretKey::generate(),
        1,
    );

    expect_tx_revert!(
        withdraw_unstaked,
        &update_socket,
        ExecutionError::NotNodeOwner
    );
}

#[tokio::test]
async fn test_withdraw_unstaked_reverts_no_locked_tokens() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let amount: HpUfixed<18> = 1_000u64.into();

    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &amount,
        &node_pub_key,
        [0; 96].into()
    );

    assert_eq!(get_staked(&query_runner, &node_pub_key), amount);

    let withdraw_unstaked =
        prepare_withdraw_unstaked_update(&node_pub_key, None, &owner_secret_key, 3);

    expect_tx_revert!(
        withdraw_unstaked,
        &update_socket,
        ExecutionError::NoLockedTokens
    );
}

#[tokio::test]
async fn test_withdraw_unstaked_works_properly() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let owner: EthAddress = owner_secret_key.to_pk().into();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let amount: HpUfixed<18> = 1_000u64.into();

    // Stake
    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &amount,
        &node_pub_key,
        [0; 96].into()
    );
    assert_eq!(get_staked(&query_runner, &node_pub_key), amount);

    // Unstake
    let update = prepare_unstake_update(&amount, &node_pub_key, &owner_secret_key, 3);
    expect_tx_success!(update, &update_socket);

    // Wait 5 epochs to unlock lock_time (5)
    for epoch in 0..5 {
        simple_epoch_change!(&update_socket, &keystore, &query_runner, epoch);
    }

    let prev_balance = get_flk_balance(&query_runner, &owner);

    //Withdraw Unstaked
    let withdraw_unstaked =
        prepare_withdraw_unstaked_update(&node_pub_key, Some(owner), &owner_secret_key, 4);
    expect_tx_success!(withdraw_unstaked, &update_socket);

    // Assert updated Flk balance
    assert_eq!(
        get_flk_balance(&query_runner, &owner),
        prev_balance + amount
    );

    // Assert reset the nodes locked stake state
    assert_eq!(
        query_runner
            .get_node_info::<HpUfixed<18>>(
                &query_runner.pubkey_to_index(&node_pub_key).unwrap(),
                |n| n.stake.locked
            )
            .unwrap(),
        HpUfixed::zero()
    );
}
