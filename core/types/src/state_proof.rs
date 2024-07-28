use std::collections::BTreeSet;
use std::time::Duration;

use atomo::SerdeBackend;
use fleek_crypto::{ClientPublicKey, ConsensusPublicKey, EthAddress, NodePublicKey};
use hp_fixed::unsigned::HpUfixed;
use serde::{Deserialize, Serialize};

use crate::{
    AccountInfo,
    Blake3Hash,
    Committee,
    CommodityTypes,
    Epoch,
    Metadata,
    NodeIndex,
    NodeInfo,
    NodeServed,
    ProtocolParams,
    ReportedReputationMeasurements,
    Service,
    ServiceId,
    ServiceRevenue,
    TotalServed,
    TxHash,
    Value,
};

#[derive(Debug, Eq, PartialEq, Serialize, Deserialize, Clone, schemars::JsonSchema)]
pub enum StateProofKey {
    Metadata(Metadata),
    Accounts(EthAddress),
    ClientKeys(ClientPublicKey),
    Nodes(NodeIndex),
    ConsensusKeyToIndex(ConsensusPublicKey),
    PubKeyToIndex(NodePublicKey),
    Latencies((NodeIndex, NodeIndex)),
    Committees(Epoch),
    Services(ServiceId),
    Parameters(ProtocolParams),
    ReputationMeasurements(NodeIndex),
    ReputationScores(NodeIndex),
    SubmittedReputationMeasurements(NodeIndex),
    CurrentEpochServed(NodeIndex),
    LastEpochServed(NodeIndex),
    TotalServed(Epoch),
    CommodityPrices(CommodityTypes),
    ServiceRevenues(ServiceId),
    ExecutedDigests(TxHash),
    Uptime(NodeIndex),
    UriToNode(Blake3Hash),
    NodeToUri(NodeIndex),
}

#[derive(Debug, Serialize, Deserialize, Clone, schemars::JsonSchema)]
#[allow(clippy::large_enum_variant)]
pub enum StateProofValue {
    Metadata(Value),
    Accounts(AccountInfo),
    ClientKeys(EthAddress),
    Nodes(NodeInfo),
    ConsensusKeyToIndex(NodeIndex),
    PubKeyToIndex(NodeIndex),
    Latencies(Duration),
    Committees(Committee),
    Services(Service),
    Parameters(u128),
    ReputationMeasurements(Vec<ReportedReputationMeasurements>),
    ReputationScores(u8),
    SubmittedReputationMeasurements(u8),
    CurrentEpochServed(NodeServed),
    LastEpochServed(NodeServed),
    TotalServed(TotalServed),
    CommodityPrices(HpUfixed<6>),
    ServiceRevenues(ServiceRevenue),
    ExecutedDigests(()),
    Uptime(u8),
    UriToNode(BTreeSet<NodeIndex>),
    NodeToUri(BTreeSet<Blake3Hash>),
}

impl StateProofKey {
    /// Returns the table name and serialized key value as a pair.
    pub fn get_raw<S: SerdeBackend>(&self) -> (String, Vec<u8>) {
        let (table, key) = match self {
            Self::Metadata(key) => ("metadata", S::serialize(key)),
            Self::Accounts(key) => ("account", S::serialize(key)),
            Self::ClientKeys(key) => ("client_keys", S::serialize(key)),
            Self::Nodes(key) => ("node", S::serialize(key)),
            Self::ConsensusKeyToIndex(key) => ("consensus_key_to_index", S::serialize(key)),
            Self::PubKeyToIndex(key) => ("pub_key_to_index", S::serialize(key)),
            Self::Latencies(key) => ("latencies", S::serialize(key)),
            Self::Committees(key) => ("committee", S::serialize(key)),
            Self::Services(key) => ("service", S::serialize(key)),
            Self::Parameters(key) => ("parameter", S::serialize(key)),
            Self::ReputationMeasurements(key) => ("rep_measurements", S::serialize(key)),
            Self::ReputationScores(key) => ("rep_scores", S::serialize(key)),
            Self::SubmittedReputationMeasurements(key) => {
                ("submitted_rep_measurements", S::serialize(key))
            },
            Self::CurrentEpochServed(key) => ("current_epoch_served", S::serialize(key)),
            Self::LastEpochServed(key) => ("last_epoch_served", S::serialize(key)),
            Self::TotalServed(key) => ("total_served", S::serialize(key)),
            Self::CommodityPrices(key) => ("commodity_prices", S::serialize(key)),
            Self::ServiceRevenues(key) => ("service_revenue", S::serialize(key)),
            Self::ExecutedDigests(key) => ("executed_digests", S::serialize(key)),
            Self::Uptime(key) => ("uptime", S::serialize(key)),
            Self::UriToNode(key) => ("uri_to_node", S::serialize(key)),
            Self::NodeToUri(key) => ("node_to_uri", S::serialize(key)),
        };
        (table.to_string(), key)
    }

    /// Returns the deserialized value for the given table/key.
    pub fn value<S: SerdeBackend>(&self, value: Vec<u8>) -> StateProofValue {
        match self {
            Self::Metadata(_) => StateProofValue::Metadata(S::deserialize(&value)),
            Self::Accounts(_) => StateProofValue::Accounts(S::deserialize(&value)),
            Self::ClientKeys(_) => StateProofValue::ClientKeys(S::deserialize(&value)),
            Self::Nodes(_) => StateProofValue::Nodes(S::deserialize(&value)),
            Self::ConsensusKeyToIndex(_) => {
                StateProofValue::ConsensusKeyToIndex(S::deserialize(&value))
            },
            Self::PubKeyToIndex(_) => StateProofValue::PubKeyToIndex(S::deserialize(&value)),
            Self::Latencies(_) => StateProofValue::Latencies(S::deserialize(&value)),
            Self::Committees(_) => StateProofValue::Committees(S::deserialize(&value)),
            Self::Services(_) => StateProofValue::Services(S::deserialize(&value)),
            Self::Parameters(_) => StateProofValue::Parameters(S::deserialize(&value)),
            Self::ReputationMeasurements(_) => {
                StateProofValue::ReputationMeasurements(S::deserialize(&value))
            },
            Self::ReputationScores(_) => StateProofValue::ReputationScores(S::deserialize(&value)),
            Self::SubmittedReputationMeasurements(_) => {
                StateProofValue::SubmittedReputationMeasurements(S::deserialize(&value))
            },
            Self::CurrentEpochServed(_) => {
                StateProofValue::CurrentEpochServed(S::deserialize(&value))
            },
            Self::LastEpochServed(_) => StateProofValue::LastEpochServed(S::deserialize(&value)),
            Self::TotalServed(_) => StateProofValue::TotalServed(S::deserialize(&value)),
            Self::CommodityPrices(_) => StateProofValue::CommodityPrices(S::deserialize(&value)),
            Self::ServiceRevenues(_) => StateProofValue::ServiceRevenues(S::deserialize(&value)),
            Self::ExecutedDigests(_) => {
                S::deserialize::<()>(&value);
                StateProofValue::ExecutedDigests(())
            },
            Self::Uptime(_) => StateProofValue::Uptime(S::deserialize(&value)),
            Self::UriToNode(_) => StateProofValue::UriToNode(S::deserialize(&value)),
            Self::NodeToUri(_) => StateProofValue::NodeToUri(S::deserialize(&value)),
        }
    }
}
