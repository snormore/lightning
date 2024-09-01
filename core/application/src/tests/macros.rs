/// Helper macro executing single Update within a single Block.
/// Asserts that submission occurred.
/// Transaction Result may be Success or Revert - `TransactionResponse`.
///
///  # Arguments
///
/// * `update: UpdateRequest` - The update request to be executed.
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
///
/// # Returns
///
/// * `BlockExecutionResponse`
macro_rules! run_update {
    ($update:expr,$socket:expr) => {{
        let updates = vec![$update.into()];
        run_transactions!(updates, $socket)
    }};
}

pub(crate) use run_update;

/// Helper macro executing many Updates within a single Block.
/// Asserts that submission occurred.
/// Transaction Result may be Success or Revert - `TransactionResponse`.
///
///  # Arguments
///
/// * `updates: Vec<UpdateRequest>` - Vector of update requests to be executed.
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
///
/// # Returns
///
/// * `BlockExecutionResponse`
macro_rules! run_updates {
    ($updates:expr,$socket:expr) => {{
        let txs = $updates.into_iter().map(|update| update.into()).collect();
        run_transactions!(txs, $socket)
    }};
}

pub(crate) use run_updates;

/// Helper macro executing many Transactions within a single Block.
/// Asserts that submission occurred.
/// Transaction Result may be Success or Revert.
///
///  # Arguments
///
/// * `txs: Vec<TransactionRequest>` - Vector of transaction to be executed.
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
///
/// # Returns
///
/// * `BlockExecutionResponse`
macro_rules! run_transactions {
    ($txs:expr,$socket:expr) => {{
        let result = super::utils::run_transaction($txs, $socket).await;
        assert!(result.is_ok());
        result.unwrap()
    }};
}

pub(crate) use run_transactions;

/// Helper macro executing a single Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Update was successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `update: UpdateRequest` - Vector of update requests to be executed.
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `response: ExecutionData` - Expected execution data, optional param
///
/// # Returns
///
/// * `BlockExecutionResponse`
macro_rules! expect_tx_success {
    ($update:expr,$socket:expr) => {{
        expect_tx_success!(
            $update,
            $socket,
            lightning_interfaces::types::ExecutionData::None
        );
    }};
    ($update:expr,$socket:expr,$response:expr) => {{
        let result = run_update!($update, $socket);
        assert_eq!(
            result.txn_receipts[0].response,
            lightning_interfaces::types::TransactionResponse::Success($response)
        );
        result
    }};
}

pub(crate) use expect_tx_success;

/// Helper macro executing a single Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Update was reverted - `TransactionResponse::Revert`.
///
///  # Arguments
///
/// * `update: UpdateRequest` - Vector of update requests to be executed.
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `revert: ExecutionError` - Expected execution error
macro_rules! expect_tx_revert {
    ($update:expr,$socket:expr,$revert:expr) => {{
        let result = run_update!($update, $socket);
        assert_eq!(
            result.txn_receipts[0].response,
            lightning_interfaces::types::TransactionResponse::Revert($revert)
        );
    }};
}

pub(crate) use expect_tx_revert;

/// Helper macro executing `ChangeEpoch` Update within a single Block.
/// Asserts that submission occurred.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &NodeSecretKey` - Node's secret key for signing transaction.
/// * `nonce: u64` - Nonce for Node's account.
/// * `epoch: u64` - Epoch to be changed.
///
/// # Returns
///
/// * `BlockExecutionResponse`
macro_rules! change_epoch {
    ($socket:expr,$secret_key:expr,$nonce:expr,$epoch:expr) => {{
        let req = prepare_update_request_node(
            UpdateMethod::ChangeEpoch { epoch: $epoch },
            $secret_key,
            $nonce,
        );
        run_update!(req, $socket)
    }};
}

pub(crate) use change_epoch;

/// Helper macro that performs an epoch change.
/// Asserts that submission occurred.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `committee_keystore: &Vec<GenesisCommitteeKeystore> ` - Keystore with committee's private
///   keys.
/// * `query_runner: &QueryRunner` - Query Runner.
/// * `epoch: u64` - Epoch to be changed.
macro_rules! simple_epoch_change {
    ($socket:expr,$committee_keystore:expr,$query_runner:expr,$epoch:expr) => {{
        let required_signals = calculate_required_signals($committee_keystore.len());
        // make call epoch change for 2/3rd committee members
        for (index, node) in $committee_keystore
            .iter()
            .enumerate()
            .take(required_signals)
        {
            let nonce = get_node_nonce($query_runner, &node.node_secret_key.to_pk()) + 1;
            let req = prepare_change_epoch_request($epoch, &node.node_secret_key, nonce);

            let res = run_update!(req, $socket);
            // check epoch change
            if index == required_signals - 1 {
                assert!(res.change_epoch);
            }
        }
    }};
}

pub(crate) use simple_epoch_change;

/// Helper macro executing `SubmitReputationMeasurements` Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Update was successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &NodeSecretKey` - Node's secret key for signing transaction.
/// * `nonce: u64` - Nonce for Node's account.
/// * `measurements: BTreeMap<u32, ReputationMeasurements>` - Reputation measurements to be
///   submitted.
macro_rules! submit_reputation_measurements {
    ($socket:expr,$secret_key:expr,$nonce:expr,$measurements:expr) => {{
        let req = prepare_update_request_node(
            UpdateMethod::SubmitReputationMeasurements {
                measurements: $measurements,
            },
            $secret_key,
            $nonce,
        );
        expect_tx_success!(req, $socket)
    }};
}

pub(crate) use submit_reputation_measurements;

/// Helper macro executing `SubmitReputationMeasurements` Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Update was successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &AccountOwnerSecretKey` - Account's secret key for signing transaction.
/// * `nonce: u64` - Nonce for the account.
/// * `amount: &HpUfixed<18>` - Amount to be deposited.
macro_rules! deposit {
    ($socket:expr,$secret_key:expr,$nonce:expr,$amount:expr) => {{
        let req = prepare_deposit_update($amount, $secret_key, $nonce);
        expect_tx_success!(req, $socket)
    }};
}

pub(crate) use deposit;

/// Helper macro executing `Stake` Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Update was successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &AccountOwnerSecretKey` - Account's secret key for signing transaction.
/// * `nonce: u64` - Nonce for the account.
/// * `amount: &HpUfixed<18>` - Amount to be staked.
/// * `node_pk: &NodePublicKey` - Public key of a Node to be staked on.
/// * `consensus_key: ConsensusPublicKey` - Consensus public key.
macro_rules! stake {
    ($socket:expr,$secret_key:expr,$nonce:expr,$amount:expr,$node_pk:expr,$consensus_key:expr) => {{
        let req = prepare_initial_stake_update(
            $amount,
            $node_pk,
            $consensus_key,
            "127.0.0.1".parse().unwrap(),
            [0; 32].into(),
            "127.0.0.1".parse().unwrap(),
            lightning_interfaces::types::NodePorts::default(),
            $secret_key,
            $nonce,
        );

        expect_tx_success!(req, $socket)
    }};
}

pub(crate) use stake;

/// Helper macro executing `Deposit` and `Stake` Updates within a single Block.
/// Asserts that submission occurred.
/// Asserts that Updates were successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &AccountOwnerSecretKey` - Account's secret key for signing transaction.
/// * `nonce: u64` - Nonce for the account.
/// * `amount: &HpUfixed<18>` - Amount to be deposited and staked.
/// * `node_pk: &NodePublicKey` - Public key of a Node to be staked on.
/// * `consensus_key: ConsensusPublicKey` - Consensus public key.
macro_rules! deposit_and_stake {
    ($socket:expr,$secret_key:expr,$nonce:expr,$amount:expr,$node_pk:expr,$consensus_key:expr) => {{
        deposit!($socket, $secret_key, $nonce, $amount);
        stake!(
            $socket,
            $secret_key,
            $nonce + 1,
            $amount,
            $node_pk,
            $consensus_key
        );
    }};
}

pub(crate) use deposit_and_stake;

/// Helper macro executing `StakeLock` Update within a single Block.
/// Asserts that submission occurred.
/// Asserts that the Updates was successful - `TransactionResponse::Success`.
///
///  # Arguments
///
/// * `socket: &ExecutionEngineSocket` - Socket for submitting transaction.
/// * `secret_key: &AccountOwnerSecretKey` - Account's secret key for signing transaction.
/// * `nonce: u64` - Nonce for the account.
/// * `node_pk: &NodePublicKey` - Public key of a Node.
/// * `locked_for: u64` - Lock time.
macro_rules! stake_lock {
    ($socket:expr,$secret_key:expr,$nonce:expr,$node_pk:expr,$locked_for:expr) => {{
        let req = prepare_stake_lock_request($locked_for, $node_pk, $secret_key, $nonce);
        expect_tx_success!(req, $socket)
    }};
}

pub(crate) use stake_lock;

/// Assert that Reputation Measurements are submitted (updated).
///
///  # Arguments
///
/// * `query_runner: &QueryRunner` - Query Runner.
/// * `update: (u32, ReputationMeasurements)` - Tuple containing node index and reputation
///   measurements.
/// * `reporting_node_index: u64` - Reporting Node index.
macro_rules! assert_rep_measurements_update {
    ($query_runner:expr,$update:expr,$reporting_node_index:expr) => {{
        let rep_measurements = $query_runner
            .get_reputation_measurements(&$update.0)
            .unwrap();
        assert_eq!(rep_measurements.len(), 1);
        assert_eq!(rep_measurements[0].reporting_node, $reporting_node_index);
        assert_eq!(rep_measurements[0].measurements, $update.1);
    }};
}

pub(crate) use assert_rep_measurements_update;

/// Assert that a Node is valid.
///
///  # Arguments
///
/// * `valid_nodes: &Vec<NodeInfo>` - List of valid nodes.
/// * `query_runner: &QueryRunner` - Query Runner.
/// * `node_pk: &NodePublicKey` - Node's public key
macro_rules! assert_valid_node {
    ($valid_nodes:expr,$query_runner:expr,$node_pk:expr) => {{
        let node_info = get_node_info($query_runner, $node_pk);
        // Node registry contains the first valid node
        assert!($valid_nodes.contains(&node_info));
    }};
}

pub(crate) use assert_valid_node;

/// Assert that a Node is NOT valid.
///
///  # Arguments
///
/// * `valid_nodes: &Vec<NodeInfo>` - List of valid nodes.
/// * `query_runner: &QueryRunner` - Query Runner.
/// * `node_pk: &NodePublicKey` - Node's public key
macro_rules! assert_not_valid_node {
    ($valid_nodes:expr,$query_runner:expr,$node_pk:expr) => {{
        let node_info = get_node_info($query_runner, $node_pk);
        // Node registry contains the first valid node
        assert!(!$valid_nodes.contains(&node_info));
    }};
}

pub(crate) use assert_not_valid_node;

/// Assert that paging works properly with `get_node_registry`.
///
///  # Arguments
///
/// * `query_runner: &QueryRunner` - Query Runner.
/// * `paging_params: PagingParams` - Paging params.
/// * `expected_len: usize` - Expected length of the query result.
macro_rules! assert_paging_node_registry {
    ($query_runner:expr,$paging_params:expr, $expected_len:expr) => {{
        let valid_nodes = $query_runner.get_node_registry(Some($paging_params));
        assert_eq!(valid_nodes.len(), $expected_len);
    }};
}

pub(crate) use assert_paging_node_registry;
