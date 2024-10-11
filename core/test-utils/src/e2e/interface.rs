use std::any::Any;
use std::collections::{BTreeSet, HashMap};

use anyhow::Result;
use fleek_crypto::{
    AccountOwnerPublicKey,
    AccountOwnerSecretKey,
    ConsensusPublicKey,
    ConsensusSecretKey,
    EthAddress,
    NodePublicKey,
    NodeSecretKey,
};
use hp_fixed::unsigned::HpUfixed;
use lightning_checkpointer::CheckpointerQuery;
use lightning_committee_beacon::CommitteeBeaconQuery;
use lightning_interfaces::types::{
    Blake3Hash,
    ChainId,
    CheckpointAttestation,
    CommitteeSelectionBeaconCommit,
    CommitteeSelectionBeaconPhase,
    CommitteeSelectionBeaconReveal,
    Epoch,
    EpochInfo,
    ExecuteTransactionError,
    ExecuteTransactionOptions,
    Genesis,
    Metadata,
    NodeIndex,
    NodeInfo,
    ProtocolParamKey,
    ProtocolParamValue,
    ReportedReputationMeasurements,
    Service,
    ServiceId,
    TransactionReceipt,
    TransactionRequest,
    UpdateMethod,
    Value,
};
use lightning_interfaces::Events;
use lightning_rep_collector::MyReputationReporter;
use lightning_rpc::RpcClient;
use lightning_utils::transaction::TransactionSigner;
use merklize::StateRootHash;

use super::TestNodeBeforeGenesisReadyState;

pub type BoxedNode = Box<dyn NetworkNode>;

#[async_trait::async_trait]
pub trait NetworkNode: Any {
    fn as_any(&self) -> &dyn Any;
    fn index(&self) -> NodeIndex;

    async fn shutdown(self: Box<Self>);

    async fn transaction_client(
        &self,
        signer: TransactionSigner,
    ) -> Box<dyn NetworkTransactionClient>;
    async fn node_transaction_client(&self) -> Box<dyn NetworkTransactionClient>;

    fn application_query(&self) -> Box<dyn NetworkQueryRunner>;
    fn checkpointer_query(&self) -> CheckpointerQuery;
    fn committee_beacon_query(&self) -> CommitteeBeaconQuery;

    fn reputation_reporter(&self) -> MyReputationReporter;

    fn rpc_client(&self) -> Result<RpcClient>;
    async fn rpc_admin_client(&self) -> Result<RpcClient>;
    async fn rpc_ws_client(&self) -> Result<jsonrpsee::ws_client::WsClient>;
    fn rpc_event_tx(&self) -> Events;

    fn emit_epoch_changed_notification(
        &self,
        epoch: Epoch,
        previous_state_root: StateRootHash,
        new_state_root: StateRootHash,
        last_epoch_hash: [u8; 32],
    );

    fn get_node_secret_key(&self) -> NodeSecretKey;
    fn get_node_public_key(&self) -> NodePublicKey;
    fn get_consensus_secret_key(&self) -> ConsensusSecretKey;
    fn get_consensus_public_key(&self) -> ConsensusPublicKey;
    fn get_owner_secret_key(&self) -> AccountOwnerSecretKey;
    fn get_owner_public_key(&self) -> AccountOwnerPublicKey;

    async fn apply_genesis(&self, genesis: Genesis) -> Result<bool>;

    async fn wait_for_before_genesis_ready(&self);
    fn get_before_genesis_ready_state(&self) -> Option<TestNodeBeforeGenesisReadyState>;
    async fn wait_for_after_genesis_ready(&self);

    async fn get_pool_connected_peers(&self) -> Result<Vec<NodeIndex>>;

    async fn broadcast_checkpoint_attestation(&self, header: CheckpointAttestation) -> Result<()>;
}

// pub trait NetworkNodeKeystoreExt {
//     fn get_node_secret_key(&self) -> NodeSecretKey;
//     fn get_consensus_secret_key(&self) -> ConsensusSecretKey;
//     fn get_owner_secret_key(&self) -> AccountOwnerSecretKey;
//     fn get_owner_public_key(&self) -> AccountOwnerPublicKey;
// }

// pub trait NetworkNodeReadyExt {
//     async fn wait_for_before_genesis_ready(&self);
//     fn get_before_genesis_ready_state(&self) -> Option<TestNodeBeforeGenesisReadyState>;
//     async fn wait_for_after_genesis_ready(&self);
// }

// pub trait NetworkNodePoolExt {
//     async fn get_pool_connected_peers(&self) -> Result<Vec<NodeIndex>>;
// }

#[async_trait::async_trait]
pub trait NetworkTransactionClient {
    async fn execute_transaction(
        &self,
        method: UpdateMethod,
        options: Option<ExecuteTransactionOptions>,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError>;

    async fn deposit_and_stake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError>;
    async fn stake_lock(
        &self,
        locked_for: u64,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError>;
    async fn unstake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError>;
}

// TODO(snormore): Can we remove the <C> param type from SyncQueryRunnerInterface and use that
// directly instead?
pub trait NetworkQueryRunner {
    fn get_chain_id(&self) -> ChainId;
    fn get_epoch(&self) -> Epoch;
    fn get_epoch_info(&self) -> EpochInfo;
    fn get_node_info(&self, node: NodeIndex) -> Option<NodeInfo>;
    fn get_metadata(&self, metadata: &Metadata) -> Option<Value>;
    fn get_protocol_param(&self, key: &ProtocolParamKey) -> Option<ProtocolParamValue>;
    fn get_committee_members(&self, epoch: Epoch) -> Option<Vec<NodeIndex>>;
    fn get_protocol_fund_address(&self) -> EthAddress;
    fn get_total_supply(&self) -> HpUfixed<18>;
    fn get_supply_year_start(&self) -> HpUfixed<18>;
    fn get_stake(&self, node: NodeIndex) -> HpUfixed<18>;
    fn get_node_nonce(&self, node: NodeIndex) -> u64;
    fn get_account_nonce(&self, account: EthAddress) -> u64;
    fn get_stables_balance(&self, account: EthAddress) -> HpUfixed<6>;
    fn get_flk_balance(&self, account: EthAddress) -> HpUfixed<18>;
    fn get_staking_amount(&self) -> u64;
    fn get_service_info(&self, service: &ServiceId) -> Option<Service>;
    fn get_uri_providers(&self, uri: &Blake3Hash) -> Option<BTreeSet<NodeIndex>>;
    fn get_content_registry(&self, node_index: &NodeIndex) -> Option<BTreeSet<Blake3Hash>>;
    fn get_reputation_score(&self, node: &NodeIndex) -> Option<u8>;
    fn get_committee_selection_beacon_phase(&self) -> Option<CommitteeSelectionBeaconPhase>;
    fn get_committee_selection_beacons(
        &self,
    ) -> HashMap<
        NodeIndex,
        (
            CommitteeSelectionBeaconCommit,
            Option<CommitteeSelectionBeaconReveal>,
        ),
    >;
    fn get_reputation_measurements(
        &self,
        node: &NodeIndex,
    ) -> Option<Vec<ReportedReputationMeasurements>>;
}
