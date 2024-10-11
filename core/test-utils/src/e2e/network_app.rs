use std::time::Duration;

use futures::future::join_all;
use lightning_interfaces::types::{Epoch, UpdateMethod};
use lightning_utils::poll::{poll_until, PollUntilError};

use super::{BoxedNode, TestNetwork};

impl TestNetwork {
    /// Execute epoch change transaction from all nodes and wait for epoch to be incremented.
    pub async fn change_epoch_and_wait_for_complete(&self) -> Result<Epoch, PollUntilError> {
        // Execute epoch change transaction from all nodes.
        let new_epoch = self.change_epoch().await;

        // Wait for epoch to be incremented across all nodes.
        self.wait_for_epoch_change(new_epoch).await?;

        // Return the new epoch.
        Ok(new_epoch)
    }

    pub async fn change_epoch(&self) -> Epoch {
        let epoch = self.node(0).application_query().get_epoch();
        join_all(self.nodes().map(|node| async {
            node.node_transaction_client()
                .await
                .execute_transaction(UpdateMethod::ChangeEpoch { epoch }, None)
                .await
        }))
        .await;
        epoch + 1
    }

    pub async fn wait_for_epoch_change(&self, new_epoch: Epoch) -> Result<(), PollUntilError> {
        poll_until(
            || async {
                self.nodes()
                    .all(|node| node.application_query().get_epoch() == new_epoch)
                    .then_some(())
                    .ok_or(PollUntilError::ConditionNotSatisfied)
            },
            Duration::from_secs(15),
            Duration::from_millis(100),
        )
        .await
    }

    pub fn committee_nodes(&self) -> Vec<&BoxedNode> {
        let query = self.node(0).application_query();
        let epoch = query.get_epoch();
        query
            .get_committee_members(epoch)
            .unwrap_or_default()
            .into_iter()
            .map(|index| self.node(index))
            .collect()
    }

    pub fn non_committee_nodes(&self) -> Vec<&BoxedNode> {
        let query = self.node(0).application_query();
        let epoch = query.get_epoch();
        let committee_nodes = query.get_committee_members(epoch).unwrap_or_default();
        self.nodes()
            .filter(|node| !committee_nodes.contains(&node.index()))
            .collect()
    }
}
