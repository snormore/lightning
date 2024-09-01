use std::collections::{BTreeMap, HashMap};
use std::net::IpAddr;
use std::str::FromStr;
use std::time::SystemTime;

use affair::Socket;
use anyhow::{anyhow, Result};
use fleek_crypto::{
    AccountOwnerSecretKey,
    ConsensusPublicKey,
    ConsensusSecretKey,
    EthAddress,
    NodePublicKey,
    NodeSecretKey,
    SecretKey,
};
use hp_fixed::signed::HpFixed;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::prelude::*;
use lightning_interfaces::{Node, PagingParams};
use lightning_test_utils::json_config::JsonConfigProvider;
use lightning_utils::application::QueryRunnerExt;
use tempfile::TempDir;
use types::{
    AccountInfo,
    Blake3Hash,
    Block,
    BlockExecutionResponse,
    ChainId,
    CommodityTypes,
    ContentUpdate,
    DeliveryAcknowledgmentProof,
    Epoch,
    HandshakePorts,
    NodeIndex,
    NodeInfo,
    NodePorts,
    Participation,
    ProofOfConsensus,
    ProtocolParams,
    ReputationMeasurements,
    Staking,
    Tokens,
    TransactionRequest,
    UpdateMethod,
    UpdatePayload,
    UpdateRequest,
};

use crate::app::Application;
use crate::config::Config;
use crate::genesis::{Genesis, GenesisAccount, GenesisNode, GenesisPrices, GenesisService};
use crate::state::QueryRunner;

partial!(TestBinding {
    ConfigProviderInterface = JsonConfigProvider;
    ApplicationInterface = Application<Self>;
});

pub const CHAIN_ID: ChainId = 1337;

pub struct Params {
    pub epoch_time: Option<u64>,
    pub max_inflation: Option<u16>,
    pub protocol_share: Option<u16>,
    pub node_share: Option<u16>,
    pub service_builder_share: Option<u16>,
    pub max_boost: Option<u16>,
    pub supply_at_genesis: Option<u64>,
}

/// Helper struct for keeping track of a node's private keys.
#[derive(Clone)]
pub struct GenesisCommitteeKeystore {
    _owner_secret_key: AccountOwnerSecretKey,
    pub node_secret_key: NodeSecretKey,
    pub consensus_secret_key: ConsensusSecretKey,
    _worker_secret_key: NodeSecretKey,
}

/// Prepare Genesis Node's Ports
pub fn test_genesis_ports(index: u16) -> NodePorts {
    let base: u16 = index * 10000;
    NodePorts {
        primary: base + 4310,
        worker: base + 4311,
        mempool: base + 4210,
        rpc: base + 4230,
        pool: base + 4300,
        pinger: base + 4350,
        handshake: HandshakePorts {
            http: base + 4220,
            webrtc: base + 4320,
            webtransport: base + 4321,
        },
    }
}

/// Prepare Test Genesis
pub fn test_genesis() -> Genesis {
    let genesis_node_owner =
        EthAddress::from_str("0x959807B8D94B324A74117956731F09E2893aCd72").unwrap();
    let domain = "127.0.0.1".parse().unwrap();

    let node_pub_key_1 =
        NodePublicKey::from_str("F5tV4PLSzx1Lt4mYBe13aYQ8hsLMTCfjgY2pLr82AumH").unwrap();
    let consensus_key_1 = ConsensusPublicKey::from_str("u76G7q22Qc5nRC5Fi6dzbNE7FQxqRKEtTS9qjDftWFwhBKmoozGLv8wFiFmGnYDFMEKyYxozWRdM3wgjs1Na3fvxDARxi9CSNJUZJfPXC2WUu3uLnUw96jPBRp7rtHEzS5H").unwrap();
    let node_pub_key_2 =
        NodePublicKey::from_str("Qt1DzUoTEn7n4itYpAPhaDsXhcriuXe1e7n7uZztGfg").unwrap();
    let consensus_key_2 = ConsensusPublicKey::from_str("21cq5icj1pWKk9DBwdUFSW6nqBrwtzHtvKWcLNrqFRxZV8UbdKYzdoSs8C1u7s7M4FKADDqsHnxETh56hdSK2Z65nsbW3xME1fNcT1s8dfHwCFk567mV4fmSSgH73mTe1H3a").unwrap();
    let node_pub_key_3 =
        NodePublicKey::from_str("8XT8Kb1PCd2kzmwHLQ8Nw9aAuKJin6tihaPQBCyf6ymn").unwrap();
    let consensus_key_3 = ConsensusPublicKey::from_str("uNHES9wjYK3HkbcPWrBiQQZ2NmcfBVKke8tbH9X7RFT9ZvjL5f55FfPvpjh2RWpxTRyMhKAxYG42TaRv2RGyEZkYcx2aJMfgPYqYaiT8KC1EPHzJYgVmYc7z2ER69LNWC7r").unwrap();
    let node_pub_key_4 =
        NodePublicKey::from_str("DA3mDUC5y7s5dNF4bL5MfTy7TtXjwt16rtspGuJcwZHS").unwrap();
    let consensus_key_4 = ConsensusPublicKey::from_str("rnSokyL9vj1cnxsrHVmuMCP677Ns4Xh6N5FmfKvjinxVCA9W8w6DiqXSQTX92TtoapS5eqcHCuKnNKamqxh5MnLpHGZ9UkjKUPWsc7hnQXqQobHTXdw1GSh88wEir94mEba").unwrap();

    let test_staking = Staking {
        staked: HpUfixed::<18>::from(1000u32),
        stake_locked_until: 0,
        locked: HpUfixed::<18>::zero(),
        locked_until: 0,
    };

    let genesis_nodes: Vec<GenesisNode> = vec![
        (node_pub_key_1, consensus_key_1),
        (node_pub_key_2, consensus_key_2),
        (node_pub_key_3, consensus_key_3),
        (node_pub_key_4, consensus_key_4),
    ]
    .iter()
    .enumerate()
    .map(|(pos, (pub_key, consensus_key))| {
        GenesisNode::new(
            genesis_node_owner,
            *pub_key,
            domain,
            *consensus_key,
            domain,
            *pub_key,
            test_genesis_ports(pos as u16 + 1),
            Some(test_staking.clone()),
            true,
        )
    })
    .collect();

    let protocol_address =
        EthAddress::from_str("0x2a8cf657769c264b0c7f88e3a716afdeaec1c318").unwrap();

    Genesis {
        chain_id: CHAIN_ID,
        epoch_start: 1684276288383,
        epoch_time: 120000,
        committee_size: 10,
        node_count: 10,
        min_stake: 1000,
        eligibility_time: 1,
        lock_time: 5,
        protocol_share: 0,
        node_share: 80,
        service_builder_share: 20,
        max_inflation: 10,
        consumer_rebate: 0,
        max_boost: 4,
        // 1460 days(epoch) meaning 4 years
        max_lock_time: 1460,
        // Set to 1 million for testing, to be determined when initial allocations are set
        supply_at_genesis: 1000000,
        min_num_measurements: 2,
        protocol_fund_address: protocol_address,
        governance_address: protocol_address,
        node_info: genesis_nodes,
        service: vec![
            GenesisService {
                id: 0,
                owner: EthAddress::from_str("0xDC0A31F9eeb151f82BF1eE6831095284fC215Ee7").unwrap(),
                commodity_type: CommodityTypes::Bandwidth,
            },
            GenesisService {
                id: 1,
                owner: EthAddress::from_str("0x684166BDbf530a256d7c92Fa0a4128669aFd9B9F").unwrap(),
                commodity_type: CommodityTypes::Compute,
            },
        ],
        account: vec![GenesisAccount {
            public_key: genesis_node_owner,
            flk_balance: HpUfixed::<18>::from(100690000000000000000u128),
            stables_balance: 100,
            bandwidth_balance: 100,
        }],
        client: HashMap::new(),
        commodity_prices: vec![
            GenesisPrices {
                commodity: CommodityTypes::Bandwidth,
                price: 0.1,
            },
            GenesisPrices {
                commodity: CommodityTypes::Compute,
                price: 0.2,
            },
        ],
        total_served: HashMap::new(),
        latencies: None,
    }
}

/// Initialize application state with provided or default configuration.
pub fn init_app(
    temp_dir: &TempDir,
    config: Option<Config>,
) -> (ExecutionEngineSocket, QueryRunner) {
    let config = config.or_else(|| {
        let genesis_path = test_genesis()
            .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
            .unwrap();
        Some(Config::test(genesis_path))
    });
    do_init_app(config.unwrap())
}

/// Initialize application with provided configuration.
pub fn do_init_app(config: Config) -> (ExecutionEngineSocket, QueryRunner) {
    let node = Node::<TestBinding>::init_with_provider(
        fdi::Provider::default()
            .with(JsonConfigProvider::default().with::<Application<TestBinding>>(config)),
    )
    .expect("failed to initialize node");

    let app = node.provider.get::<Application<TestBinding>>();
    (app.transaction_executor(), app.sync_query())
}

/// Initialize application with provided committee.
pub fn test_init_app(
    temp_dir: &TempDir,
    committee: Vec<GenesisNode>,
) -> (ExecutionEngineSocket, QueryRunner) {
    let mut genesis = test_genesis();
    genesis.node_info = committee;
    let genesis_path = genesis
        .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
        .unwrap();
    init_app(temp_dir, Some(Config::test(genesis_path)))
}

/// Initialize application with provided genesis.
pub fn init_app_with_genesis(
    temp_dir: &TempDir,
    genesis: &Genesis,
) -> (ExecutionEngineSocket, QueryRunner) {
    let genesis_path = genesis
        .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
        .unwrap();
    init_app(temp_dir, Some(Config::test(genesis_path)))
}

/// Initialize application with provided parameters.
pub fn init_app_with_params(
    temp_dir: &TempDir,
    params: Params,
    committee: Option<Vec<GenesisNode>>,
) -> (ExecutionEngineSocket, QueryRunner) {
    let mut genesis = test_genesis();

    if let Some(committee) = committee {
        genesis.node_info = committee;
    }

    genesis.epoch_start = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    if let Some(epoch_time) = params.epoch_time {
        genesis.epoch_time = epoch_time;
    }

    if let Some(max_inflation) = params.max_inflation {
        genesis.max_inflation = max_inflation;
    }

    if let Some(protocol_share) = params.protocol_share {
        genesis.protocol_share = protocol_share;
    }

    if let Some(node_share) = params.node_share {
        genesis.node_share = node_share;
    }

    if let Some(service_builder_share) = params.service_builder_share {
        genesis.service_builder_share = service_builder_share;
    }

    if let Some(max_boost) = params.max_boost {
        genesis.max_boost = max_boost;
    }

    if let Some(supply_at_genesis) = params.supply_at_genesis {
        genesis.supply_at_genesis = supply_at_genesis;
    }

    let genesis_path = genesis
        .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
        .unwrap();

    init_app(temp_dir, Some(Config::test(genesis_path)))
}

/// Prepare test Reputation Measurements based on provided `uptime`.
pub fn test_reputation_measurements(uptime: u8) -> ReputationMeasurements {
    ReputationMeasurements {
        latency: None,
        interactions: None,
        inbound_bandwidth: None,
        outbound_bandwidth: None,
        bytes_received: None,
        bytes_sent: None,
        uptime: Some(HpFixed::from(uptime as i32)),
        hops: None,
    }
}

/// Calculate requited signals for epoch change
pub fn calculate_required_signals(committee_size: usize) -> usize {
    2 * committee_size / 3 + 1
}

/// Create a test genesis committee.
pub fn create_genesis_committee(
    num_members: usize,
) -> (Vec<GenesisNode>, Vec<GenesisCommitteeKeystore>) {
    let mut keystore = Vec::new();
    let mut committee = Vec::new();
    (0..num_members as u16).for_each(|i| {
        let node_secret_key = NodeSecretKey::generate();
        let consensus_secret_key = ConsensusSecretKey::generate();
        let owner_secret_key = AccountOwnerSecretKey::generate();
        let node = create_committee_member(
            &owner_secret_key,
            &node_secret_key,
            &consensus_secret_key,
            i,
        );
        committee.push(node);
        keystore.push(GenesisCommitteeKeystore {
            _owner_secret_key: owner_secret_key,
            _worker_secret_key: node_secret_key.clone(),
            node_secret_key,
            consensus_secret_key,
        });
    });
    (committee, keystore)
}

/// Create a new member for test committee.
pub fn create_committee_member(
    owner_secret_key: &AccountOwnerSecretKey,
    node_secret_key: &NodeSecretKey,
    consensus_secret_key: &ConsensusSecretKey,
    index: u16,
) -> GenesisNode {
    let node_public_key = node_secret_key.to_pk();
    let consensus_public_key = consensus_secret_key.to_pk();
    let owner_public_key = owner_secret_key.to_pk();
    GenesisNode::new(
        owner_public_key.into(),
        node_public_key,
        "127.0.0.1".parse().unwrap(),
        consensus_public_key,
        "127.0.0.1".parse().unwrap(),
        node_public_key,
        NodePorts {
            primary: 8000 + index,
            worker: 9000 + index,
            mempool: 7000 + index,
            rpc: 6000 + index,
            pool: 5000 + index,
            pinger: 2000 + index,
            handshake: HandshakePorts {
                http: 5000 + index,
                webrtc: 6000 + index,
                webtransport: 7000 + index,
            },
        },
        None,
        true,
    )
}

/// Prepare an `UpdateRequest` from an `UpdateMethod` signed with `NodeSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_update_request_node(
    method: UpdateMethod,
    secret_key: &NodeSecretKey,
    nonce: u64,
) -> UpdateRequest {
    let payload = UpdatePayload {
        sender: secret_key.to_pk().into(),
        nonce,
        method,
        chain_id: CHAIN_ID,
    };
    let digest = payload.to_digest();
    let signature = secret_key.sign(&digest);
    UpdateRequest {
        signature: signature.into(),
        payload,
    }
}

/// Prepare an `UpdateRequest` from an `UpdateMethod` signed with `ConsensusSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_update_request_consensus(
    method: UpdateMethod,
    secret_key: &ConsensusSecretKey,
    nonce: u64,
) -> UpdateRequest {
    let payload = UpdatePayload {
        sender: secret_key.to_pk().into(),
        nonce,
        method,
        chain_id: CHAIN_ID,
    };
    let digest = payload.to_digest();
    let signature = secret_key.sign(&digest);
    UpdateRequest {
        signature: signature.into(),
        payload,
    }
}

/// Prepare an `UpdateRequest` from an `UpdateMethod` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_update_request_account(
    method: UpdateMethod,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    let payload = UpdatePayload {
        sender: secret_key.to_pk().into(),
        nonce,
        method,
        chain_id: CHAIN_ID,
    };
    let digest = payload.to_digest();
    let signature = secret_key.sign(&digest);
    UpdateRequest {
        signature: signature.into(),
        payload,
    }
}

/// Prepare an `UpdateRequest` for `UpdateMethod::Deposit` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_deposit_update(
    amount: &HpUfixed<18>,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::Deposit {
            proof: ProofOfConsensus {},
            token: Tokens::FLK,
            amount: amount.clone(),
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::Stake` signed with `AccountOwnerSecretKey`.
/// For the first `Stake`, use `prepare_initial_stake_update`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_regular_stake_update(
    amount: &HpUfixed<18>,
    node_public_key: &NodePublicKey,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::Stake {
            amount: amount.clone(),
            node_public_key: *node_public_key,
            consensus_key: None,
            node_domain: None,
            worker_public_key: None,
            worker_domain: None,
            ports: None,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::Stake` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
#[allow(clippy::too_many_arguments)]
pub fn prepare_initial_stake_update(
    amount: &HpUfixed<18>,
    node_public_key: &NodePublicKey,
    consensus_key: ConsensusPublicKey,
    node_domain: IpAddr,
    worker_pub_key: NodePublicKey,
    worker_domain: IpAddr,
    ports: NodePorts,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::Stake {
            amount: amount.clone(),
            node_public_key: *node_public_key,
            consensus_key: Some(consensus_key),
            node_domain: Some(node_domain),
            worker_public_key: Some(worker_pub_key),
            worker_domain: Some(worker_domain),
            ports: Some(ports),
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::Unstake` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_unstake_update(
    amount: &HpUfixed<18>,
    node_public_key: &NodePublicKey,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::Unstake {
            amount: amount.clone(),
            node: *node_public_key,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::WithdrawUnstaked` signed with
/// `AccountOwnerSecretKey`. Passing the private key around like this should only be done for
/// testing.
pub fn prepare_withdraw_unstaked_update(
    node_public_key: &NodePublicKey,
    recipient: Option<EthAddress>,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::WithdrawUnstaked {
            node: *node_public_key,
            recipient,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::StakeLock` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_stake_lock_update(
    node_public_key: &NodePublicKey,
    locked_for: u64,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::StakeLock {
            node: *node_public_key,
            locked_for,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::SubmitDeliveryAcknowledgmentAggregation` signed
/// with `NodeSecretKey`. Passing the private key around like this should only be done for testing.
pub fn prepare_pod_request(
    commodity: u128,
    service_id: u32,
    secret_key: &NodeSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_node(
        UpdateMethod::SubmitDeliveryAcknowledgmentAggregation {
            commodity,  // units of data served
            service_id, // service 0 serving bandwidth
            proofs: vec![DeliveryAcknowledgmentProof],
            metadata: None,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::SubmitDeliveryAcknowledgmentAggregation` signed
/// with `AccountOwnerSecretKey`. Passing the private key around like this should only be done for
/// testing.
pub fn prepare_stake_lock_request(
    locked_for: u64,
    node: &NodePublicKey,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    // Deposit some FLK into account 1
    prepare_update_request_account(
        UpdateMethod::StakeLock {
            node: *node,
            locked_for,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::ChangeEpoch` signed with `NodeSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_change_epoch_request(
    epoch: u64,
    secret_key: &NodeSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_node(UpdateMethod::ChangeEpoch { epoch }, secret_key, nonce)
}

/// Prepare an `UpdateRequest` for `UpdateMethod::Transfer` signed with `AccountOwnerSecretKey`.
/// Passing the private key around like this should only be done for testing.
pub fn prepare_transfer_request(
    amount: &HpUfixed<18>,
    to: &EthAddress,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::Transfer {
            amount: amount.clone(),
            token: Tokens::FLK,
            to: *to,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::ChangeProtocolParam` signed with
/// `AccountOwnerSecretKey`. Passing the private key around like this should only be done for
/// testing.
pub fn prepare_change_protocol_param_request(
    param: &ProtocolParams,
    value: &u128,
    secret_key: &AccountOwnerSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_account(
        UpdateMethod::ChangeProtocolParam {
            param: param.clone(),
            value: *value,
        },
        secret_key,
        nonce,
    )
}

/// Prepare an `UpdateRequest` for `UpdateMethod::UpdateContentRegistry` signed with
/// `NodeSecretKey`. Passing the private key around like this should only be done for testing.
pub fn prepare_content_registry_update(
    updates: Vec<ContentUpdate>,
    secret_key: &NodeSecretKey,
    nonce: u64,
) -> UpdateRequest {
    prepare_update_request_node(
        UpdateMethod::UpdateContentRegistry { updates },
        secret_key,
        nonce,
    )
}

/// Helper (async) function that submit a transaction to the application via `UpdateSocket`.
/// Returns `Result<BlockExecutionResponse>`.
pub async fn run_transaction(
    requests: Vec<TransactionRequest>,
    update_socket: &Socket<Block, BlockExecutionResponse>,
) -> Result<BlockExecutionResponse> {
    let res = update_socket
        .run(Block {
            transactions: requests,
            digest: [0; 32],
            sub_dag_index: 0,
            sub_dag_round: 0,
        })
        .await
        .map_err(|r| anyhow!(format!("{r:?}")))?;
    Ok(res)
}

/// Helper function that update `BTreeMap<u32, ReputationMeasurements>` with new
/// `ReputationMeasurements` for given `NodePublicKey` Returns tuple `(peer_index, measurements)`.
pub fn update_reputation_measurements(
    query_runner: &QueryRunner,
    map: &mut BTreeMap<u32, ReputationMeasurements>,
    peer: &NodePublicKey,
    measurements: ReputationMeasurements,
) -> (u32, ReputationMeasurements) {
    let peer_index = get_node_index(query_runner, peer);
    map.insert(peer_index, measurements.clone());
    (peer_index, measurements)
}

/// Helper function that prepare `PagingParams`
pub fn paging_params(ignore_stake: bool, start: u32, limit: usize) -> PagingParams {
    PagingParams {
        ignore_stake,
        start,
        limit,
    }
}

/// Helper function that add a node to the `committee`.
pub fn add_to_committee(
    committee: &mut Vec<GenesisNode>,
    keystore: &mut Vec<GenesisCommitteeKeystore>,
    node_secret_key: NodeSecretKey,
    consensus_secret_key: ConsensusSecretKey,
    owner_secret_key: AccountOwnerSecretKey,
    index: u16,
) {
    let node_public_key = node_secret_key.to_pk();
    let consensus_public_key = consensus_secret_key.to_pk();
    let owner_public_key = owner_secret_key.to_pk();
    committee.push(GenesisNode::new(
        owner_public_key.into(),
        node_public_key,
        "127.0.0.1".parse().unwrap(),
        consensus_public_key,
        "127.0.0.1".parse().unwrap(),
        node_public_key,
        NodePorts {
            primary: 8000 + index,
            worker: 9000 + index,
            mempool: 7000 + index,
            rpc: 6000 + index,
            pool: 5000 + index,
            pinger: 2000 + index,
            handshake: HandshakePorts {
                http: 5000 + index,
                webrtc: 6000 + index,
                webtransport: 7000 + index,
            },
        },
        None,
        true,
    ));
    keystore.push(GenesisCommitteeKeystore {
        _owner_secret_key: owner_secret_key,
        _worker_secret_key: node_secret_key.clone(),
        node_secret_key,
        consensus_secret_key,
    });
}

/// Helper function that prepare new `committee`.
pub fn prepare_new_committee(
    query_runner: &QueryRunner,
    committee: &[GenesisNode],
    keystore: &[GenesisCommitteeKeystore],
) -> (Vec<GenesisNode>, Vec<GenesisCommitteeKeystore>) {
    let mut new_committee = Vec::new();
    let mut new_keystore = Vec::new();
    let committee_members = query_runner.get_committee_members();
    for node in committee_members {
        let index = committee
            .iter()
            .enumerate()
            .find_map(|(index, c)| {
                if c.primary_public_key == node {
                    Some(index)
                } else {
                    None
                }
            })
            .expect("Committee member was not found in genesis Committee");
        new_committee.push(committee[index].clone());
        new_keystore.push(keystore[index].clone());
    }
    (new_committee, new_keystore)
}

/// Convert NodePublicKey to NodeIndex
pub fn get_node_index(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> NodeIndex {
    query_runner.pubkey_to_index(pub_key).unwrap()
}

/// Query NodeTable
pub fn do_get_node_info<T: Clone>(
    query_runner: &QueryRunner,
    pub_key: &NodePublicKey,
    selector: impl FnOnce(NodeInfo) -> T,
) -> T {
    let node_idx = get_node_index(query_runner, pub_key);
    query_runner
        .get_node_info::<T>(&node_idx, selector)
        .unwrap()
}

/// Query NodeInfo from NodeTable
pub fn get_node_info(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> NodeInfo {
    do_get_node_info(query_runner, pub_key, |n| n)
}

/// Query Node's Nonce from NodeTable
pub fn get_node_nonce(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> u64 {
    do_get_node_info::<u64>(query_runner, pub_key, |n| n.nonce)
}

/// Query Node's Participation from NodeTable
pub fn get_node_participation(
    query_runner: &QueryRunner,
    pub_key: &NodePublicKey,
) -> Participation {
    do_get_node_info::<Participation>(query_runner, pub_key, |n| n.participation)
}

/// Query Node's Stake amount
pub fn get_staked(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> HpUfixed<18> {
    do_get_node_info::<HpUfixed<18>>(query_runner, pub_key, |n| n.stake.staked)
}

/// Query Node's Locked amount
pub fn get_locked(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> HpUfixed<18> {
    do_get_node_info::<HpUfixed<18>>(query_runner, pub_key, |n| n.stake.locked)
}

/// Query Node's Locked amount
pub fn get_locked_time(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> Epoch {
    do_get_node_info::<Epoch>(query_runner, pub_key, |n| n.stake.locked_until)
}

/// Query Node's stake locked until
pub fn get_stake_locked_until(query_runner: &QueryRunner, pub_key: &NodePublicKey) -> Epoch {
    do_get_node_info::<Epoch>(query_runner, pub_key, |n| n.stake.stake_locked_until)
}

/// Query AccountInfo from AccountTable
pub fn do_get_account_info<T: Clone>(
    query_runner: &QueryRunner,
    address: &EthAddress,
    selector: impl FnOnce(AccountInfo) -> T,
) -> Option<T> {
    query_runner.get_account_info::<T>(address, selector)
}

/// Query Account's Flk balance
pub fn get_flk_balance(query_runner: &QueryRunner, address: &EthAddress) -> HpUfixed<18> {
    do_get_account_info::<HpUfixed<18>>(query_runner, address, |a| a.flk_balance)
        .unwrap_or(HpUfixed::<18>::zero())
}

/// Query Account's bandwidth balance
pub fn get_account_balance(query_runner: &QueryRunner, address: &EthAddress) -> u128 {
    do_get_account_info::<u128>(query_runner, address, |a| a.bandwidth_balance).unwrap_or(0)
}

/// Query Account's stables balance
pub fn get_stables_balance(query_runner: &QueryRunner, address: &EthAddress) -> HpUfixed<6> {
    do_get_account_info::<HpUfixed<6>>(query_runner, address, |a| a.stables_balance)
        .unwrap_or(HpUfixed::<6>::zero())
}

pub fn uri_to_providers(query_runner: &QueryRunner, uri: &Blake3Hash) -> Vec<NodeIndex> {
    query_runner
        .get_uri_providers(uri)
        .unwrap_or_default()
        .into_iter()
        .collect()
}

pub fn content_registry(query_runner: &QueryRunner, node: &NodeIndex) -> Vec<Blake3Hash> {
    query_runner
        .get_content_registry(node)
        .unwrap_or_default()
        .into_iter()
        .collect()
}
