use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;
use lightning_application::state::QueryRunner;
use lightning_interfaces::types::Genesis;
use lightning_utils::poll::{poll_until, PollUntilError};

use super::{BoxedNode, TestGenesisBuilder, TestNetwork, TestNode, TestNodeComponents};
use crate::consensus::{Config as MockConsensusConfig, MockConsensusGroup};

pub type GenesisMutator = Arc<dyn Fn(&mut Genesis)>;

pub struct TestNetworkBuilder {
    pub num_nodes: u32,
    pub committee_size: u32,
    pub genesis_mutator: Option<GenesisMutator>,
    pub mock_consensus_config: Option<MockConsensusConfig>,
}

impl TestNetworkBuilder {
    pub fn new() -> Self {
        Self {
            num_nodes: 3,
            committee_size: 3,
            genesis_mutator: None,
            mock_consensus_config: Some(Self::default_mock_consensus_config()),
        }
    }

    pub fn with_num_nodes(mut self, num_nodes: u32) -> Self {
        self.num_nodes = num_nodes;
        self
    }

    pub fn with_committee_size(mut self, committee_size: u32) -> Self {
        self.committee_size = committee_size;
        self
    }

    pub fn with_genesis_mutator<F>(mut self, mutator: F) -> Self
    where
        F: Fn(&mut Genesis) + 'static,
    {
        self.genesis_mutator = Some(Arc::new(mutator));
        self
    }

    pub fn with_mock_consensus(mut self, config: MockConsensusConfig) -> Self {
        self.mock_consensus_config = Some(config);
        self
    }

    pub fn without_mock_consensus(mut self) -> Self {
        self.mock_consensus_config = None;
        self
    }

    pub fn default_mock_consensus_config() -> MockConsensusConfig {
        MockConsensusConfig {
            max_ordering_time: 1,
            min_ordering_time: 0,
            probability_txn_lost: 0.0,
            new_block_interval: Duration::from_secs(0),
            ..Default::default()
        }
    }

    pub fn new_mock_consensus_group(
        config: Option<MockConsensusConfig>,
    ) -> (MockConsensusGroup, Arc<tokio::sync::Notify>) {
        let config = config.unwrap_or_else(Self::default_mock_consensus_config);
        let notify = Arc::new(tokio::sync::Notify::new());
        let consensus_group =
            MockConsensusGroup::new::<QueryRunner>(config, None, Some(notify.clone()));
        (consensus_group, notify)
    }

    pub async fn build(&self) -> Result<TestNetwork> {
        // TODO(snormore): Remove this when finished debugging.
        let _ = crate::e2e::try_init_tracing();

        // Build the mock consensus group.
        let (consensus_group, consensus_group_start) =
            Self::new_mock_consensus_group(self.mock_consensus_config.clone());

        // Build and start the non-customized nodes.
        let nodes = join_all((0..self.num_nodes).map(|_| {
            TestNode::<TestNodeComponents>::builder()
                .with_mock_consensus(Some(consensus_group.clone()))
                .build::<TestNodeComponents>()
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        // Build the network with the nodes.
        self.build_with_nodes(nodes, Some(consensus_group_start))
            .await
    }

    /// Builds a new test network with the given number of nodes, and starts each of them.
    pub async fn build_with_nodes(
        &self,
        nodes: Vec<BoxedNode>,
        // TODO(snormore): Move this into BuildOptions or something.
        consensus_group_start: Option<Arc<tokio::sync::Notify>>,
    ) -> Result<TestNetwork> {
        // TODO(snormore): Remove this when finished debugging.
        let _ = crate::e2e::try_init_tracing();

        // Wait for ready before building genesis.
        join_all(
            nodes
                .iter()
                .map(|node| node.wait_for_before_genesis_ready()),
        )
        .await;

        // Decide which nodes will be on the genesis committee.
        let node_by_index = nodes.iter().enumerate().collect::<HashMap<_, _>>();
        let committee_nodes = node_by_index
            .iter()
            .take(self.committee_size as usize)
            .collect::<HashMap<_, _>>();

        // Build genesis.
        let genesis = {
            let mut builder = TestGenesisBuilder::default();
            if let Some(mutator) = self.genesis_mutator.clone() {
                builder = builder.with_mutator(mutator);
            }
            for (node_index, node) in &node_by_index {
                builder = builder.with_node(node, committee_nodes.contains_key(&node_index));
            }
            builder.build()
        };

        // Apply genesis on each node.
        join_all(nodes.iter().map(|node| node.apply_genesis(genesis.clone()))).await;

        // Wait for the pool to establish all of the node connections.
        self.wait_for_connected_peers(&nodes).await?;

        // Wait for ready after genesis.
        join_all(nodes.iter().map(|node| node.wait_for_after_genesis_ready())).await;

        // Notify the shared mock consensus group that it can start.
        if let Some(consensus_group_start) = &consensus_group_start {
            consensus_group_start.notify_waiters();
        }

        let network = TestNetwork::new(genesis, nodes).await?;
        Ok(network)
    }

    pub async fn wait_for_connected_peers(
        &self,
        nodes: &[BoxedNode],
    ) -> Result<(), PollUntilError> {
        poll_until(
            || async {
                let peers_by_node =
                    join_all(nodes.iter().map(|node| node.get_pool_connected_peers()))
                        .await
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| PollUntilError::ConditionNotSatisfied)?;

                tracing::debug!(
                    "waiting for connected peers (nodes: {}): {:?}",
                    nodes.len(),
                    peers_by_node
                        .iter()
                        .map(|peers| peers.len())
                        .collect::<Vec<_>>()
                );

                peers_by_node
                    .iter()
                    .all(|peers| peers.len() == nodes.len() - 1)
                    .then_some(())
                    .ok_or(PollUntilError::ConditionNotSatisfied)
            },
            Duration::from_secs(30),
            Duration::from_millis(200),
        )
        .await
    }
}

impl Default for TestNetworkBuilder {
    fn default() -> Self {
        Self::new()
    }
}
