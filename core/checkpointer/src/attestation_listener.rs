use std::collections::{HashMap, HashSet};

use anyhow::Result;
use atomo::{DefaultSerdeBackend, SerdeBackend};
use bit_set::BitSet;
use fleek_crypto::{
    ConsensusAggregateSignature,
    ConsensusPublicKey,
    ConsensusSignature,
    PublicKey,
};
use lightning_interfaces::prelude::*;
use lightning_interfaces::{spawn, BroadcastInterface, Collection, PubSub, ShutdownWaiter};
use lightning_utils::application::QueryRunnerExt;
use tokio::task::JoinHandle;
use types::{Epoch, NodeIndex};

use crate::database::CheckpointerDatabase;
use crate::database_rocks::RocksCheckpointerDatabase;
use crate::headers::{AggregateCheckpointHeader, CheckpointHeader};
use crate::message::CheckpointBroadcastMessage;

/// The attestation listener is responsible for listening for checkpoint attestation
/// messages and saving them to the local database.
///
/// When a supermajority of attestations for epochs are consistent, it aggregates the BLS
/// signatures to create a canonical aggregate checkpoint header, which is saves to the local
/// database for sharing with other nodes and clients in the future.
pub struct AttestationListener<C: Collection> {
    node_id: NodeIndex,
    db: RocksCheckpointerDatabase,
    pubsub: <C::BroadcastInterface as BroadcastInterface<C>>::PubSub<CheckpointBroadcastMessage>,
    app_query: c!(C::ApplicationInterface::SyncExecutor),
}

impl<C: Collection> AttestationListener<C> {
    pub fn new(
        node_id: NodeIndex,
        db: RocksCheckpointerDatabase,
        // pubsub: <C::BroadcastInterface as BroadcastInterface<C>>::PubSub<
        //     CheckpointBroadcastMessage,
        // >,
        pubsub: c!(C::BroadcastInterface::PubSub<CheckpointBroadcastMessage>),
        app_query: c!(C::ApplicationInterface::SyncExecutor),
    ) -> Self {
        Self {
            node_id,
            db,
            pubsub,
            app_query,
        }
    }

    /// Spawn task for and start the attestation listener.
    ///
    /// This method spawns a new task and returns immediately. It does not block
    /// until the task is complete.
    pub fn spawn(self, shutdown: ShutdownWaiter) -> JoinHandle<Result<()>> {
        spawn!(
            async move {
                shutdown
                    .run_until_shutdown(self.start())
                    .await
                    .unwrap_or(Ok(()))
            },
            "CHECKPOINTER: attestation listener"
        )
    }

    pub async fn start(mut self) -> Result<()> {
        tracing::debug!("starting attestation listener");

        // Listen for checkpoint attestation messages.
        loop {
            tokio::select! {
                Some(msg) = self.pubsub.recv() => {
                    tracing::debug!("received checkpoint attestation message: {:?}", msg);
                    match msg {
                        CheckpointBroadcastMessage::CheckpointHeader(checkpoint_header) => {
                            self.handle_incoming_checkpoint_header(checkpoint_header)?;
                        }
                    }
                }
                else => {
                    // TODO(snormore): What should actually happen here?
                    tracing::debug!("no more messages");
                    break;
                }
            }
        }

        tracing::debug!("shutdown attestation listener");
        Ok(())
    }

    fn handle_incoming_checkpoint_header(
        &mut self,
        checkpoint_header: CheckpointHeader,
    ) -> Result<()> {
        let epoch = checkpoint_header.epoch;
        let nodes = self.app_query.get_active_nodes();
        let node_consensus_key = nodes
            .iter()
            .find(|node| node.index == checkpoint_header.node_id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Node {} not found in active nodes",
                    checkpoint_header.node_id
                )
            })?
            .info
            .consensus_key;

        // Save the incoming checkpoint header attestation to the database.
        self.validate_checkpoint_header(&checkpoint_header, node_consensus_key)?;
        self.db
            .add_checkpoint_header(epoch, checkpoint_header.clone())?;

        // Check if we can build an aggregate checkpoint header for the epoch.
        let aggr_header = self.db.get_aggregate_checkpoint_header(epoch)?;
        match aggr_header {
            Some(_) => {
                // There is already an aggregate checkpoint header in the database for this epoch,
                // we don't need to process any more checkpoint headers for this
                // epoch.
            },
            None => {
                // Get the number of active nodes from the application query runner.
                let nodes_count = nodes.len();

                // Check for supermajority of checkpoint headers for the epoch.
                // If found, aggregate the signatures and save an aggregate checkpoint header to the
                // local database.
                self.check_for_supermajority(epoch, nodes_count)?;
            },
        }

        Ok(())
    }

    fn validate_checkpoint_header(
        &self,
        header: &CheckpointHeader,
        node_consensus_key: ConsensusPublicKey,
    ) -> Result<()> {
        let serialized_signed_header = DefaultSerdeBackend::serialize(&CheckpointHeader {
            signature: ConsensusSignature::default(),
            ..header.clone()
        });
        if !node_consensus_key.verify(&header.signature, &serialized_signed_header) {
            return Err(anyhow::anyhow!("Invalid checkpoint header signature"));
        }

        Ok(())
    }

    fn check_for_supermajority(&self, epoch: Epoch, nodes_count: usize) -> Result<()> {
        // Get checkpoint headers from the database for the epoch.
        let headers = self.db.get_checkpoint_headers(epoch)?;

        // Check if we have a supermajority of attestations that are in agreement for the
        // epoch.
        let mut headers_by_state_root = HashMap::new();
        for header in headers.iter() {
            headers_by_state_root
                .entry(header.next_state_root)
                .or_insert_with(HashSet::new)
                .insert(header);
            let state_root_headers = &headers_by_state_root[&header.next_state_root];

            // We assume signatures have been validated and deduplicated.

            if state_root_headers.len() > (2 * nodes_count) / 3 {
                // Get the node's own checkpoint header for the epoch.
                let node_header = headers
                    .iter()
                    .find(|header| header.node_id == self.node_id)
                    .ok_or_else(|| {
                        anyhow::anyhow!("node's own checkpoint header not found in database.")
                    })?;

                // We have a supermajority of attestations in agreement for the epoch.
                let aggregate_header =
                    self.build_aggregate_checkpoint_header(epoch, node_header, state_root_headers)?;

                // Save the aggregate signature to the local database.
                self.db
                    .set_aggregate_checkpoint_header(epoch, aggregate_header)?;

                break;
            }
        }

        Ok(())
    }

    fn build_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
        node_header: &CheckpointHeader,
        state_root_headers: &HashSet<&CheckpointHeader>,
    ) -> Result<AggregateCheckpointHeader> {
        // Aggregate the signatures.
        let signatures = state_root_headers
            .iter()
            .map(|header| header.signature)
            .collect::<Vec<_>>();
        let aggregate_signature = ConsensusAggregateSignature::aggregate(signatures.iter())?;

        // TODO(snormore): What should happen if the node's own checkpoint header
        // is not found? Where does the previous state root come from if we continue
        // with it? If we fail, should we do it in a way that retries, expecting
        // that our our epoch change listener hasn't triggered yet while it has for
        // other nodes, and that ours will eventually? If it doesn't we miss out on
        // having an aggregate checkpoint header for this epoch. Is that ok?
        // TODO(snormore): Write a test for this scenario.

        // Build the nodes bit set.
        let nodes = BitSet::<NodeIndex>::from_iter(
            state_root_headers
                .iter()
                .map(|header| header.node_id as usize),
        );

        // Create the aggregate checkpoint header.
        let aggregate_header = AggregateCheckpointHeader {
            epoch,
            // We use the previous state root from the node's own checkpoint header.
            previous_state_root: node_header.previous_state_root,
            // The next state root is equal across them all, so we can use any.
            next_state_root: node_header.next_state_root,
            signature: aggregate_signature,
            nodes,
        };

        Ok(aggregate_header)
    }
}
