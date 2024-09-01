use std::net::IpAddr;

use fleek_crypto::{
    AccountOwnerSecretKey,
    ConsensusPublicKey,
    EthAddress,
    NodePublicKey,
    NodeSecretKey,
    SecretKey,
};
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{ExecutionError, HandshakePorts, NodePorts, UpdateMethod};
use lightning_interfaces::SyncQueryRunnerInterface;
use tempfile::tempdir;

use super::macros::*;
use super::utils::*;

#[tokio::test]
async fn test_stake() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, _keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let peer_pub_key = NodeSecretKey::generate().to_pk();

    // Deposit some FLK into account 1
    let deposit = 1000_u64.into();
    let update1 = prepare_deposit_update(&deposit, &owner_secret_key, 1);
    let update2 = prepare_deposit_update(&deposit, &owner_secret_key, 2);

    // Put 2 of the transaction in the block just to also test block exucution a bit
    let _ = run_updates!(vec![update1, update2], &update_socket);

    // check that he has 2_000 flk balance
    assert_eq!(
        get_flk_balance(&query_runner, &owner_secret_key.to_pk().into()),
        (HpUfixed::<18>::from(2u16) * deposit)
    );

    // Test staking on a new node
    let stake_amount = 1000u64.into();
    // First check that trying to stake without providing all the node info reverts
    let update = prepare_regular_stake_update(&stake_amount, &peer_pub_key, &owner_secret_key, 3);
    expect_tx_revert!(
        update,
        &update_socket,
        ExecutionError::InsufficientNodeDetails
    );

    // Now try with the correct details for a new node
    let update = prepare_initial_stake_update(
        &stake_amount,
        &peer_pub_key,
        [0; 96].into(),
        "127.0.0.1".parse().unwrap(),
        [0; 32].into(),
        "127.0.0.1".parse().unwrap(),
        NodePorts::default(),
        &owner_secret_key,
        4,
    );

    expect_tx_success!(update, &update_socket);

    // Query the new node and make sure he has the proper stake
    assert_eq!(get_staked(&query_runner, &peer_pub_key), stake_amount);

    // Stake 1000 more but since it is not a new node we should be able to leave the optional
    // parameters out without a revert
    let update = prepare_regular_stake_update(&stake_amount, &peer_pub_key, &owner_secret_key, 5);

    expect_tx_success!(update, &update_socket);

    // Node should now have 2_000 stake
    assert_eq!(
        get_staked(&query_runner, &peer_pub_key),
        (HpUfixed::<18>::from(2u16) * stake_amount.clone())
    );

    // Now test unstake and make sure it moves the tokens to locked status
    let update = prepare_unstake_update(&stake_amount, &peer_pub_key, &owner_secret_key, 6);
    run_update!(update, &update_socket);

    // Check that his locked is 1000 and his remaining stake is 1000
    assert_eq!(get_staked(&query_runner, &peer_pub_key), stake_amount);
    assert_eq!(get_locked(&query_runner, &peer_pub_key), stake_amount);

    // Since this test starts at epoch 0 locked_until will be == lock_time
    assert_eq!(
        get_locked_time(&query_runner, &peer_pub_key),
        test_genesis().lock_time
    );

    // Try to withdraw the locked tokens and it should revert
    let update = prepare_withdraw_unstaked_update(&peer_pub_key, None, &owner_secret_key, 7);

    expect_tx_revert!(update, &update_socket, ExecutionError::TokensLocked);
}

#[tokio::test]
async fn test_stake_lock() {
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

    let locked_for = 365;
    let stake_lock_req = prepare_stake_lock_update(&node_pub_key, locked_for, &owner_secret_key, 3);

    expect_tx_success!(stake_lock_req, &update_socket);

    assert_eq!(
        get_stake_locked_until(&query_runner, &node_pub_key),
        locked_for
    );

    let unstake_req = prepare_unstake_update(&amount, &node_pub_key, &owner_secret_key, 4);
    expect_tx_revert!(
        unstake_req,
        &update_socket,
        ExecutionError::LockedTokensUnstakeForbidden
    );
}

#[tokio::test]
async fn test_revert_stake_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    let amount: HpUfixed<18> = 1000_u64.into();

    let stake = UpdateMethod::Stake {
        amount,
        node_public_key: keystore[0].node_secret_key.to_pk(),
        consensus_key: None,
        node_domain: None,
        worker_public_key: None,
        worker_domain: None,
        ports: None,
    };

    // Check that trying to Stake funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key = prepare_update_request_node(stake.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to Stake funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key = prepare_update_request_consensus(stake, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );
}

#[tokio::test]
async fn test_revert_stake_insufficient_balance() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, _keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let address: EthAddress = owner_secret_key.to_pk().into();

    let peer_pub_key = NodeSecretKey::generate().to_pk();

    // Deposit some FLK into an account
    let deposit = 1000_u64.into();
    deposit!(&update_socket, &owner_secret_key, 1, &deposit);

    let balance = get_flk_balance(&query_runner, &address);

    // Now try with the correct details for a new node
    let update = prepare_initial_stake_update(
        &(deposit + <u64 as Into<HpUfixed<18>>>::into(1)),
        &peer_pub_key,
        [0; 96].into(),
        "127.0.0.1".parse().unwrap(),
        [0; 32].into(),
        "127.0.0.1".parse().unwrap(),
        NodePorts::default(),
        &owner_secret_key,
        2,
    );

    // Expect Revert Error
    expect_tx_revert!(update, &update_socket, ExecutionError::InsufficientBalance);

    // Flk balance has not changed
    assert_eq!(get_flk_balance(&query_runner, &address), balance);
}

#[tokio::test]
async fn test_revert_stake_consensus_key_already_indexed() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let address: EthAddress = owner_secret_key.to_pk().into();

    let peer_pub_key = NodeSecretKey::generate().to_pk();

    // Deposit some FLK into an account
    let deposit = 1000_u64.into();
    deposit!(&update_socket, &owner_secret_key, 1, &deposit);

    let balance = get_flk_balance(&query_runner, &address);

    // Now try with the correct details for a new node
    let update = prepare_initial_stake_update(
        &deposit,
        &peer_pub_key,
        keystore[0].consensus_secret_key.to_pk(),
        "127.0.0.1".parse().unwrap(),
        [0; 32].into(),
        "127.0.0.1".parse().unwrap(),
        NodePorts::default(),
        &owner_secret_key,
        2,
    );

    // Expect Revert Error
    expect_tx_revert!(
        update,
        &update_socket,
        ExecutionError::ConsensusKeyAlreadyIndexed
    );

    // Flk balance has not changed
    assert_eq!(get_flk_balance(&query_runner, &address), balance);
}

#[tokio::test]
async fn test_stake_works() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, _keystore) = create_genesis_committee(committee_size);
    let (update_socket, query_runner) = test_init_app(&temp_dir, committee);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let address: EthAddress = owner_secret_key.to_pk().into();

    let peer_pub_key = NodeSecretKey::generate().to_pk();

    // Deposit some FLK into an account
    let stake = 1000_u64.into();
    deposit!(&update_socket, &owner_secret_key, 1, &stake);

    let balance = get_flk_balance(&query_runner, &address);
    let consensus_key: ConsensusPublicKey = [0; 96].into();
    let node_domain: IpAddr = "89.64.54.26".parse().unwrap();
    let worker_pub_key: NodePublicKey = [0; 32].into();
    let worker_domain: IpAddr = "127.0.0.1".parse().unwrap();
    let node_ports = NodePorts {
        primary: 4001,
        worker: 4002,
        mempool: 4003,
        rpc: 4004,
        pool: 4005,
        pinger: 4007,
        handshake: HandshakePorts {
            http: 5001,
            webrtc: 5002,
            webtransport: 5003,
        },
    };
    // Now try with the correct details for a new node
    let update = prepare_initial_stake_update(
        &stake,
        &peer_pub_key,
        consensus_key,
        node_domain,
        worker_pub_key,
        worker_domain,
        node_ports.clone(),
        &owner_secret_key,
        2,
    );

    // Expect Success
    expect_tx_success!(update, &update_socket);

    // Flk balance has not changed
    assert_eq!(
        get_flk_balance(&query_runner, &address),
        balance - stake.clone()
    );

    let node_info = get_node_info(&query_runner, &peer_pub_key);
    assert_eq!(node_info.consensus_key, consensus_key);
    assert_eq!(node_info.domain, node_domain);
    assert_eq!(node_info.worker_public_key, worker_pub_key);
    assert_eq!(node_info.worker_domain, worker_domain);
    assert_eq!(node_info.ports, node_ports);

    // Query the new node and make sure he has the proper stake
    assert_eq!(get_staked(&query_runner, &peer_pub_key), stake);

    let node_idx = query_runner.pubkey_to_index(&peer_pub_key).unwrap();
    assert_eq!(
        query_runner.index_to_pubkey(&node_idx).unwrap(),
        peer_pub_key
    );
}

#[tokio::test]
async fn test_stake_lock_reverts_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    let stake_lock = UpdateMethod::StakeLock {
        node: NodeSecretKey::generate().to_pk(),
        locked_for: 365,
    };

    // Check that trying to StakeLock funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key = prepare_update_request_node(stake_lock.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to StakeLock funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key =
        prepare_update_request_consensus(stake_lock, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );
}

#[tokio::test]
async fn test_stake_lock_reverts_node_does_not_exist() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, _query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let locked_for = 365;
    let stake_lock_req = prepare_stake_lock_update(&node_pub_key, locked_for, &owner_secret_key, 1);

    expect_tx_revert!(
        stake_lock_req,
        &update_socket,
        ExecutionError::NodeDoesNotExist
    );
}

#[tokio::test]
async fn test_stake_lock_reverts_not_node_owner() {
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

    let locked_for = 365;
    let stake_lock_req = prepare_stake_lock_update(
        &node_pub_key,
        locked_for,
        &AccountOwnerSecretKey::generate(),
        1,
    );

    expect_tx_revert!(stake_lock_req, &update_socket, ExecutionError::NotNodeOwner);
}

#[tokio::test]
async fn test_stake_lock_reverts_insufficient_stake() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let amount: HpUfixed<18> = 0u64.into();

    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &amount,
        &node_pub_key,
        [0; 96].into()
    );

    assert_eq!(get_staked(&query_runner, &node_pub_key), amount);

    let locked_for = 365;
    let stake_lock_req = prepare_stake_lock_update(&node_pub_key, locked_for, &owner_secret_key, 3);

    expect_tx_revert!(
        stake_lock_req,
        &update_socket,
        ExecutionError::InsufficientStake
    );
}

#[tokio::test]
async fn test_stake_lock_reverts_lock_exceeded_max_stake_lock_time() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let amount: HpUfixed<18> = 1000u64.into();

    deposit_and_stake!(
        &update_socket,
        &owner_secret_key,
        1,
        &amount,
        &node_pub_key,
        [0; 96].into()
    );

    assert_eq!(get_staked(&query_runner, &node_pub_key), amount);

    // max locked time from genesis
    let locked_for = 1460 + 1;
    let stake_lock_req = prepare_stake_lock_update(&node_pub_key, locked_for, &owner_secret_key, 3);

    expect_tx_revert!(
        stake_lock_req,
        &update_socket,
        ExecutionError::LockExceededMaxStakeLockTime
    );
}

#[tokio::test]
async fn test_unstake_reverts_not_account_key() {
    let temp_dir = tempdir().unwrap();

    let committee_size = 4;
    let (committee, keystore) = create_genesis_committee(committee_size);
    let (update_socket, _query_runner) = test_init_app(&temp_dir, committee);

    let unstake = UpdateMethod::Unstake {
        amount: 100u64.into(),
        node: NodeSecretKey::generate().to_pk(),
    };

    // Check that trying to Unstake funds with Node Key reverts
    let node_secret_key = &keystore[0].node_secret_key;
    let update_node_key = prepare_update_request_node(unstake.clone(), node_secret_key, 1);
    expect_tx_revert!(
        update_node_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );

    // Check that trying to Unstake funds with Consensus Key reverts
    let consensus_secret_key = &keystore[0].consensus_secret_key;
    let update_consensus_key = prepare_update_request_consensus(unstake, consensus_secret_key, 2);
    expect_tx_revert!(
        update_consensus_key,
        &update_socket,
        ExecutionError::OnlyAccountOwner
    );
}

#[tokio::test]
async fn test_unstake_reverts_node_does_not_exist() {
    let temp_dir = tempdir().unwrap();

    let (update_socket, _query_runner) = init_app(&temp_dir, None);

    let owner_secret_key = AccountOwnerSecretKey::generate();
    let node_pub_key = NodeSecretKey::generate().to_pk();
    let update = prepare_unstake_update(&100u64.into(), &node_pub_key, &owner_secret_key, 1);

    expect_tx_revert!(update, &update_socket, ExecutionError::NodeDoesNotExist);
}

#[tokio::test]
async fn test_unstake_reverts_insufficient_balance() {
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

    let update = prepare_unstake_update(
        &(amount + <u64 as Into<HpUfixed<18>>>::into(1)),
        &node_pub_key,
        &owner_secret_key,
        3,
    );

    expect_tx_revert!(update, &update_socket, ExecutionError::InsufficientBalance);
}
