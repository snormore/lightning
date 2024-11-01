use lightning_interfaces::schema::AutoImplSerde;
use lightning_interfaces::types::NodeRemoval;
use serde::{Deserialize, Serialize};

/// The message envelope that is broadcasted to all nodes in the network on the node removal topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeRemovalBroadcastMessage {
    // TODO(snormore): Clean this up.
    NodeRemoval(NodeRemoval),
}

impl AutoImplSerde for NodeRemovalBroadcastMessage {}
