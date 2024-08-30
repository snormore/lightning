use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use atomo::{Atomo, InMemoryStorage, KeyIterator, QueryPerm, StorageBackend};
use fdi::BuildGraph;
use fleek_crypto::{ClientPublicKey, EthAddress, NodePublicKey};
use lightning_types::{
    AccountInfo,
    Blake3Hash,
    Committee,
    NodeIndex,
    StateProofKey,
    StateProofValue,
    TransactionRequest,
    TxHash,
    Value,
};
use merklize::trees::mpt::MptStateProof;
use merklize::{StateRootHash, StateTree};
use serde::{Deserialize, Serialize};

use crate::collection::Collection;
use crate::types::{
    Epoch,
    NodeInfo,
    NodeServed,
    ProtocolParams,
    ReportedReputationMeasurements,
    Service,
    ServiceId,
    TotalServed,
    TransactionResponse,
};
use crate::ConfigConsumer;

#[interfaces_proc::blank]
pub trait ApplicationStateInterface<C: Collection>: BuildGraph + Sized + Send + Sync {
    // /// The type for the storage backend.
    // type Storage: StorageBackend;

    // /// The type for the serde backend.
    // #[blank(DefaultSerdeBackend)]
    // type Serde: SerdeBackend;

    // /// The type for the state tree.
    // type Tree: StateTree;

    /// The type for the query runner.
    type Query: SyncQueryRunnerInterface;

    /// Returns the query runner for the application state.
    fn query(&self) -> Self::Query;
}

#[interfaces_proc::blank]
pub trait SyncQueryRunnerInterface: Clone + Send + Sync + 'static {
    #[blank(InMemoryStorage)]
    type Backend: StorageBackend;

    fn new(atomo: Atomo<QueryPerm, Self::Backend>) -> Self;

    fn atomo_from_checkpoint(
        path: impl AsRef<Path>,
        hash: [u8; 32],
        checkpoint: &[u8],
    ) -> Result<Atomo<QueryPerm, Self::Backend>>;

    fn atomo_from_path(path: impl AsRef<Path>) -> Result<Atomo<QueryPerm, Self::Backend>>;

    /// Query Metadata Table
    fn get_metadata(&self, key: &lightning_types::Metadata) -> Option<Value>;

    /// Query Account Table
    /// Returns information about an account.
    fn get_account_info<V>(
        &self,
        address: &EthAddress,
        selector: impl FnOnce(AccountInfo) -> V,
    ) -> Option<V>;

    /// Query Client Table
    fn client_key_to_account_key(&self, pub_key: &ClientPublicKey) -> Option<EthAddress>;

    /// Query Node Table
    /// Returns information about a single node.
    fn get_node_info<V>(&self, node: &NodeIndex, selector: impl FnOnce(NodeInfo) -> V)
    -> Option<V>;

    /// Returns an Iterator to Node Table
    fn get_node_table_iter<V>(&self, closure: impl FnOnce(KeyIterator<NodeIndex>) -> V) -> V;

    /// Query Pub Key to Node Index Table
    fn pubkey_to_index(&self, pub_key: &NodePublicKey) -> Option<NodeIndex>;

    /// Query Committee Table
    fn get_committe_info<V>(
        &self,
        epoch: &Epoch,
        selector: impl FnOnce(Committee) -> V,
    ) -> Option<V>;

    /// Query Services Table
    /// Returns the service information for a given [`ServiceId`]
    fn get_service_info(&self, id: &ServiceId) -> Option<Service>;

    /// Query Params Table
    /// Returns the passed in protocol parameter
    fn get_protocol_param(&self, param: &ProtocolParams) -> Option<u128>;

    /// Query Current Epoch Served Table
    fn get_current_epoch_served(&self, node: &NodeIndex) -> Option<NodeServed>;

    /// Query Reputation Measurements Table
    /// Returns the reported reputation measurements for a node.
    fn get_reputation_measurements(
        &self,
        node: &NodeIndex,
    ) -> Option<Vec<ReportedReputationMeasurements>>;

    /// Query Latencies Table
    fn get_latencies(&self, nodes: &(NodeIndex, NodeIndex)) -> Option<Duration>;

    /// Returns an Iterator to Latencies Table
    fn get_latencies_iter<V>(
        &self,
        closure: impl FnOnce(KeyIterator<(NodeIndex, NodeIndex)>) -> V,
    ) -> V;

    /// Query Reputation Scores Table
    /// Returns the global reputation of a node.
    fn get_reputation_score(&self, node: &NodeIndex) -> Option<u8>;

    /// Query Total Served Table
    /// Returns total served for all commodities from the state for a given epoch
    fn get_total_served(&self, epoch: &Epoch) -> Option<TotalServed>;

    /// Checks if an transaction digest has been executed this epoch.
    fn has_executed_digest(&self, digest: TxHash) -> bool;

    /// Get Node's Public Key based on the Node's Index
    fn index_to_pubkey(&self, node_index: &NodeIndex) -> Option<NodePublicKey>;

    /// Simulate Transaction
    fn simulate_txn(&self, txn: TransactionRequest) -> TransactionResponse;

    /// Returns the uptime for a node from the past epoch.
    fn get_node_uptime(&self, node_index: &NodeIndex) -> Option<u8>;

    /// Returns nodes that are providing the content addressed by the cid.
    fn get_uri_providers(&self, uri: &Blake3Hash) -> Option<BTreeSet<NodeIndex>>;

    /// Returns the node's content registry.
    fn get_content_registry(&self, node_index: &NodeIndex) -> Option<BTreeSet<Blake3Hash>>;

    /// Returns the state root hash from the application state.
    fn get_state_root(&self) -> Result<StateRootHash>;

    /// Returns the state proof for a given key from the application state using the state tree.
    fn get_state_proof(
        &self,
        key: StateProofKey,
    ) -> Result<(Option<StateProofValue>, MptStateProof)>;
}

#[derive(Deserialize, Serialize, schemars::JsonSchema)]
pub struct PagingParams {
    // Since some nodes may be in state without
    // having staked the minimum and if at any point
    // they stake the minimum amount, this would
    // cause inconsistent results.
    // This flag allows you to query for all nodes
    // to keep returned results consistent.
    pub ignore_stake: bool,
    pub start: NodeIndex,
    pub limit: usize,
}
