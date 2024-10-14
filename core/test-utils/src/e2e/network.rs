use std::collections::HashMap;

use anyhow::Result;
use futures::future::join_all;
use lightning_interfaces::types::{Genesis, NodeIndex};

use super::{BoxedNode, TestNetworkBuilder};

/// A network of test nodes.
///
/// This encapsulates the management of nodes and provides methods to interact with them.
pub struct TestNetwork {
    pub genesis: Genesis,
    pub node_by_id: HashMap<NodeIndex, Option<BoxedNode>>,
}

impl TestNetwork {
    pub async fn new(genesis: Genesis, nodes: Vec<BoxedNode>) -> Result<Self> {
        Ok(Self {
            genesis,
            // We assume that at this point the genesis has been applied, otherwise this will panic.
            node_by_id: nodes
                .into_iter()
                .map(|node| (node.index(), Some(node)))
                .collect::<HashMap<_, _>>(),
        })
    }

    pub fn builder() -> TestNetworkBuilder {
        TestNetworkBuilder::new()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &BoxedNode> {
        self.node_by_id.values().map(|node| node.as_ref().unwrap())
    }

    pub fn maybe_node(&self, node_id: NodeIndex) -> Option<&BoxedNode> {
        self.node_by_id.get(&node_id).and_then(|node| node.as_ref())
    }

    pub fn node(&self, node_id: NodeIndex) -> &BoxedNode {
        self.maybe_node(node_id).expect("node not found")
    }

    pub fn node_count(&self) -> usize {
        self.node_by_id.len()
    }

    pub async fn shutdown(mut self) {
        // join_all(self.node_by_id.drain().map(|(_, mut node)| node.shutdown())).await;
        let shutdown_futures = self.node_by_id.iter_mut().filter_map(|(_, node_opt)| {
            // Take the node (move out of the Option) and call shutdown
            node_opt.take().map(|node| node.shutdown())
        });
        join_all(shutdown_futures).await;
    }
}
