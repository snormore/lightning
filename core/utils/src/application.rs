use std::collections::HashMap;
use std::time::Duration;

use autometrics::autometrics;
use fleek_crypto::NodePublicKey;
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{
    Epoch,
    EpochInfo,
    Metadata,
    NodeIndex,
    NodeInfo,
    NodeInfoWithIndex,
    ProtocolParams,
    Value,
};
use lightning_interfaces::PagingParams;

pub trait QueryRunnerExt: SyncQueryRunnerInterface {
    /// Returns the chain id
    fn get_chain_id(&self) -> u32 {
        match self.get_metadata(&Metadata::ChainId) {
            Some(Value::ChainId(id)) => id,
            _ => 0,
        }
    }

    /// Returns the committee members of the current epoch
    #[autometrics]
    fn get_committee_members(&self) -> Vec<NodePublicKey> {
        self.get_committee_members_by_index()
            .into_iter()
            .filter_map(|node_index| self.index_to_pubkey(&node_index))
            .collect()
    }

    /// Returns the committee members of the current epoch by NodeIndex
    fn get_committee_members_by_index(&self) -> Vec<NodeIndex> {
        let epoch = self.get_current_epoch();
        self.get_committe_info(&epoch, |c| c.members)
            .unwrap_or_default()
    }

    /// Get Current Epoch
    /// Returns just the current epoch
    fn get_current_epoch(&self) -> Epoch {
        match self.get_metadata(&Metadata::Epoch) {
            Some(Value::Epoch(epoch)) => epoch,
            _ => 0,
        }
    }

    /// Get Current Epoch Info
    /// Returns all the information on the current epoch that Narwhal needs to run
    fn get_epoch_info(&self) -> EpochInfo {
        let epoch = self.get_current_epoch();
        // look up current committee
        let committee = self.get_committe_info(&epoch, |c| c).unwrap_or_default();
        EpochInfo {
            committee: committee
                .members
                .iter()
                .filter_map(|member| self.get_node_info::<NodeInfo>(member, |n| n))
                .collect(),
            epoch,
            epoch_end: committee.epoch_end_timestamp,
        }
    }

    /// Return all latencies measurements for the current epoch.
    fn get_current_latencies(&self) -> HashMap<(NodePublicKey, NodePublicKey), Duration> {
        self.get_latencies_iter::<HashMap<(NodePublicKey, NodePublicKey), Duration>>(
            |latencies| -> HashMap<(NodePublicKey, NodePublicKey), Duration> {
                latencies
                    .filter_map(|nodes| self.get_latencies(&nodes).map(|latency| (nodes, latency)))
                    .filter_map(|((index_lhs, index_rhs), latency)| {
                        let node_lhs =
                            self.get_node_info::<NodePublicKey>(&index_lhs, |n| n.public_key);
                        let node_rhs =
                            self.get_node_info::<NodePublicKey>(&index_rhs, |n| n.public_key);
                        match (node_lhs, node_rhs) {
                            (Some(node_lhs), Some(node_rhs)) => {
                                Some(((node_lhs, node_rhs), latency))
                            },
                            _ => None,
                        }
                    })
                    .collect()
            },
        )
    }

    /// Returns the node info of the genesis committee members
    fn get_genesis_committee(&self) -> Vec<(NodeIndex, NodeInfo)> {
        match self.get_metadata(&Metadata::GenesisCommittee) {
            Some(Value::GenesisCommittee(committee)) => committee
                .iter()
                .filter_map(|node_index| {
                    self.get_node_info::<NodeInfo>(node_index, |n| n)
                        .map(|node_info| (*node_index, node_info))
                })
                .collect(),
            _ => {
                // unreachable seeded at genesis
                Vec::new()
            },
        }
    }

    /// Returns last executed block hash. [0;32] is genesis
    fn get_last_block(&self) -> [u8; 32] {
        match self.get_metadata(&Metadata::LastBlockHash) {
            Some(Value::Hash(hash)) => hash,
            _ => [0; 32],
        }
    }

    /// Returns the current sub dag index
    fn get_sub_dag_index(&self) -> u64 {
        if let Some(Value::SubDagIndex(value)) = self.get_metadata(&Metadata::SubDagIndex) {
            value
        } else {
            0
        }
    }

    /// Returns a full copy of the entire node-registry,
    /// Paging Params - filtering nodes that are still a valid node and have enough stake; Takes
    /// from starting index and specified amount.
    fn get_node_registry(&self, paging: Option<PagingParams>) -> Vec<NodeInfoWithIndex> {
        let staking_amount = self.get_staking_amount().into();

        self.get_node_table_iter::<Vec<NodeInfoWithIndex>>(|nodes| -> Vec<NodeInfoWithIndex> {
            let nodes = nodes.map(|index| NodeInfoWithIndex {
                index,
                info: self.get_node_info::<NodeInfo>(&index, |n| n).unwrap(),
            });
            match paging {
                None => nodes
                    .filter(|node| node.info.stake.staked >= staking_amount)
                    .collect(),
                Some(PagingParams {
                    ignore_stake,
                    limit,
                    start,
                }) => {
                    let mut nodes = nodes
                        .filter(|node| ignore_stake || node.info.stake.staked >= staking_amount)
                        .collect::<Vec<NodeInfoWithIndex>>();

                    nodes.sort_by_key(|info| info.index);

                    nodes
                        .into_iter()
                        .filter(|info| info.index >= start)
                        .take(limit)
                        .collect()
                },
            }
        })
    }

    /// Gets the current active node set for a given epoch
    fn get_active_nodes(&self) -> Vec<NodeInfoWithIndex> {
        let current_epoch = self.get_current_epoch();

        let node_indexes = self
            .get_committe_info(&current_epoch, |committee| committee.active_node_set)
            .unwrap_or_default();

        node_indexes
            .iter()
            .filter_map(|index| {
                self.get_node_info(index, |node_info| node_info)
                    .map(|info| NodeInfoWithIndex {
                        index: *index,
                        info,
                    })
            })
            .collect()
    }

    /// Returns the amount that is required to be a valid node in the network.
    fn get_staking_amount(&self) -> u128 {
        self.get_protocol_param(&ProtocolParams::MinimumNodeStake)
            .unwrap_or(0)
    }

    /// Returns true if the node is a valid node in the network, with enough stake.
    fn is_valid_node(&self, id: &NodePublicKey) -> bool {
        let minimum_stake_amount = self.get_staking_amount().into();
        self.pubkey_to_index(id).is_some_and(|node_idx| {
            self.get_node_info(&node_idx, |n| n.stake.staked)
                .is_some_and(|node_stake| node_stake >= minimum_stake_amount)
        })
    }
}

impl<T: SyncQueryRunnerInterface> QueryRunnerExt for T {}
