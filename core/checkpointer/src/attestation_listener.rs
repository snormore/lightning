use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use atomo::{DefaultSerdeBackend, SerdeBackend};
use bit_set::BitSet;
use fleek_crypto::{
    ConsensusAggregateSignature,
    ConsensusPublicKey,
    ConsensusSignature,
    PublicKey,
};
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{AggregateCheckpointHeader, CheckpointHeader};
use lightning_utils::application::QueryRunnerExt;
use tokio::task::JoinHandle;
use types::{Epoch, NodeIndex};

use crate::database::{CheckpointerDatabase, CheckpointerDatabaseQuery};
use crate::message::CheckpointBroadcastMessage;
use crate::rocks::RocksCheckpointerDatabase;

/// The attestation listener is responsible for listening for checkpoint attestation
/// messages and saving them to the local database.
///
/// When a supermajority of attestations for epochs are consistent, it aggregates the BLS
/// signatures to create a canonical aggregate checkpoint header, which is saves to the local
/// database for sharing with other nodes and clients in the future.
pub struct AttestationListener<C: Collection> {
    node_id: NodeIndex,
    db: RocksCheckpointerDatabase,
    pubsub: c!(C::BroadcastInterface::PubSub<CheckpointBroadcastMessage>),
    app_query: c!(C::ApplicationInterface::SyncExecutor),
}

impl<C: Collection> AttestationListener<C> {
    pub fn new(
        node_id: NodeIndex,
        db: RocksCheckpointerDatabase,
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
    pub fn spawn(self, shutdown: ShutdownWaiter) -> JoinHandle<()> {
        let waiter = shutdown.clone();
        spawn!(
            async move {
                waiter
                    .run_until_shutdown(self.start())
                    .await
                    .unwrap_or(Ok(())) // Shutdown was triggered, so we return Ok(())
                    .context("attestation listener task failed")
                    .unwrap()
            },
            "CHECKPOINTER: attestation listener",
            crucial(shutdown)
        )
    }

    // Start the attestation listener, listening for incoming checkpoint attestation messages from
    // the broadcaster pubsub topic.
    pub async fn start(mut self) -> Result<()> {
        tracing::debug!("starting attestation listener");

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
                    tracing::debug!("broadcast subscription is closed");
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

        // TODO(snormore): Should we ignore headers from nodes that are not in the active set or in
        // some other set?

        let node_consensus_key = match self
            .app_query
            .get_node_info(&checkpoint_header.node_id, |node| node.consensus_key)
        {
            Some(key) => key,
            None => {
                tracing::warn!(
                    "checkpointer header node {} not found",
                    checkpoint_header.node_id
                );
                return Ok(());
            },
        };

        // Save the incoming checkpoint header attestation to the database.
        self.validate_checkpoint_header(&checkpoint_header, node_consensus_key)?;
        self.db
            .add_checkpoint_header(epoch, checkpoint_header.clone());

        // Check if we can build an aggregate checkpoint header for the epoch.
        let aggr_header = self.db.query().get_aggregate_checkpoint_header(epoch);
        match aggr_header {
            Some(_) => {
                // There is already an aggregate checkpoint header in the database for this epoch,
                // so we don't need to process any more checkpoint headers for this epoch.
            },
            None => {
                // Get the number of active nodes from the application query runner.
                // TODO(snormore): Is this the right set of nodes to use here?
                let nodes = self.app_query.get_active_nodes();
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

    // Check if we have a supermajority of attestations that are in agreement for the epoch, and
    // build an aggregate checkpoint header, and save it to the local database.
    //
    // We assume that the checkpoint header signatures have been validated and deduplicated by the
    // time they reach this point.
    fn check_for_supermajority(&self, epoch: Epoch, nodes_count: usize) -> Result<()> {
        let headers = self.db.query().get_checkpoint_headers(epoch);

        let mut headers_by_state_root = HashMap::new();
        for header in headers.iter() {
            headers_by_state_root
                .entry(header.next_state_root)
                .or_insert_with(HashSet::new)
                .insert(header);
            let state_root_headers = &headers_by_state_root[&header.next_state_root];

            if state_root_headers.len() > (2 * nodes_count) / 3 {
                tracing::info!("checkpoint supermajority reached for epoch {}", epoch);

                // Get the node's own checkpoint header for the epoch.
                let node_header = headers
                    .iter()
                    .find(|header| header.node_id == self.node_id)
                    .unwrap_or_else(|| {
                        // TODO(snormore): We should use the previous root has from our own database
                        // here, and not just use the the one from the first header.
                        headers.iter().peekable().peek().expect("no headers found")
                    });

                // TODO(snormore): What should happen if the node's own checkpoint header
                // is not found in the database?
                // - The previous state root comes from the node's own checkpoint header, otherwise
                //   what should it be?
                // - Should we not move forward here without it?
                // - Should we move forward and use the previous state root from another node if
                //   there's a supermajority without us?
                // - Should we require that previous state roots match across the supermajority?
                // - Should we not have the previous state root on the aggregate header at all?
                // TODO(snormore): Write a test for this scenario.

                // We have a supermajority of attestations in agreement for the epoch.
                let aggregate_header = self.build_aggregate_checkpoint_header(
                    epoch,
                    // We use the previous state root from the node's own checkpoint header.
                    node_header.previous_state_root,
                    // The next state root is equal across them all, so we can use any.
                    node_header.next_state_root,
                    state_root_headers,
                )?;

                // Save the aggregate signature to the local database.
                self.db
                    .set_aggregate_checkpoint_header(epoch, aggregate_header);

                // TODO(snormore): Emit an `EpochCheckpointNotification` via the notifier.

                break;
            } else {
                tracing::debug!("missing supermajority of checkpoints for epoch {}", epoch);
            }
        }

        Ok(())
    }

    fn build_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
        previous_state_root: [u8; 32],
        next_state_root: [u8; 32],
        state_root_headers: &HashSet<&CheckpointHeader>,
    ) -> Result<AggregateCheckpointHeader> {
        // Aggregate the signatures.
        let signatures = state_root_headers
            .iter()
            .map(|header| header.signature)
            .collect::<Vec<_>>();
        let aggregate_signature = ConsensusAggregateSignature::aggregate(signatures.iter())
            .map_err(|e| anyhow::anyhow!(e))?;

        // Build the nodes bit set.
        let nodes = BitSet::<NodeIndex>::from_iter(
            state_root_headers
                .iter()
                .map(|header| header.node_id as usize),
        );

        // TODO(snormore): Get previous and current/new state root from app query. Store root for
        // each epoch in the database with the state tree so it can be looked up without
        // being an archive node.
        // TODO(snormore): Verify header's state roots match the stored state roots for previous and
        // current/new epoch.
        // let previous_state_root = self.app_query.get_state_root(epoch - 1);

        // Create the aggregate checkpoint header.
        let aggregate_header = AggregateCheckpointHeader {
            epoch,
            previous_state_root,
            next_state_root,
            signature: aggregate_signature,
            nodes,
        };

        Ok(aggregate_header)
    }
}
