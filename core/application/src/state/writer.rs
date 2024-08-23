use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::{anyhow, Result};
use atomo::{
    Atomo,
    AtomoBuilder,
    DefaultSerdeBackend,
    StorageBackendConstructor,
    TableSelector,
    UpdatePerm,
};
use fleek_crypto::{ClientPublicKey, ConsensusPublicKey, EthAddress, NodePublicKey};
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
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
use lightning_interfaces::SyncQueryRunnerInterface;
use merklize::{StateTree, StateTreeBuilder, StateTreeWriter};

use super::context::StateContext;
use super::executor::StateExecutor;
use super::query::QueryRunner;
use crate::storage::AtomoStorage;

/// The shared application state accumulates by executing transactions.
pub struct ApplicationState<T: StateTree> {
    db: Atomo<UpdatePerm, <T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
}

impl<T: StateTree> ApplicationState<T> {
    /// Creates a new application state.
    pub(crate) fn new(
        db: Atomo<UpdatePerm, <T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
    ) -> Self {
        Self { db }
    }

    /// Registers the application and state tree tables, and builds the atomo database.
    pub fn build(atomo: AtomoBuilder<T::StorageBuilder, T::Serde>) -> Result<Self> {
        let atomo = ApplicationState::register_tables(atomo);

        let db = atomo
            .build()
            .map_err(|e| anyhow!("Failed to build atomo: {:?}", e))?;

        Ok(Self::new(db))
    }

    /// Returns a reader for the application state.
    pub fn query(&self) -> QueryRunner<T::Reader> {
        QueryRunner::new(self.db.query())
    }

    /// Returns a mutable reference to the atomo storage backend.
    ///
    /// This is unsafe because it allows modifying the state tree without going through the
    /// executor, which can lead to inconsistent state across nodes.
    pub fn get_storage_backend_unsafe(
        &mut self,
    ) -> &<T::StorageBuilder as StorageBackendConstructor>::Storage {
        self.db.get_storage_backend_unsafe()
    }

    /// Returns a state executor that handles transaction execution logic, reading and modifying the
    /// state.
    pub fn executor(
        ctx: &mut TableSelector<AtomoStorage, DefaultSerdeBackend>,
    ) -> StateExecutor<StateContext<AtomoStorage, DefaultSerdeBackend>> {
        StateExecutor::new(StateContext {
            table_selector: ctx,
        })
    }

    /// Runs a mutation on the state.
    pub fn run<F, R>(&mut self, mutation: F) -> Result<R>
    where
        F: FnOnce(&mut TableSelector<AtomoStorage, DefaultSerdeBackend>) -> R,
    {
        self.db.run(|ctx| {
            let result = mutation(ctx);

            <T::Writer as StateTreeWriter<T>>::update_state_tree_from_context(ctx)?;

            Ok(result)
        })
    }

    /// Clear and rebuild the state tree.
    /// This is namespaced as unsafe because it acts directly on the storage backend, bypassing the
    /// safety and consistency of atomo.
    pub fn clear_and_rebuild_state_tree_unsafe(&mut self) -> Result<()> {
        <T::Writer as StateTreeWriter<T>>::clear_and_rebuild_state_tree_unsafe(&mut self.db)
    }

    /// Registers and configures the application state tables with the atomo database builder.
    pub fn register_tables(
        builder: AtomoBuilder<T::StorageBuilder, T::Serde>,
    ) -> AtomoBuilder<T::StorageBuilder, T::Serde> {
        let mut builder = builder
            .with_table::<Metadata, Value>("metadata")
            .with_table::<EthAddress, AccountInfo>("account")
            .with_table::<ClientPublicKey, EthAddress>("client_keys")
            .with_table::<NodeIndex, NodeInfo>("node")
            .with_table::<ConsensusPublicKey, NodeIndex>("consensus_key_to_index")
            .with_table::<NodePublicKey, NodeIndex>("pub_key_to_index")
            .with_table::<(NodeIndex, NodeIndex), Duration>("latencies")
            .with_table::<Epoch, Committee>("committee")
            .with_table::<ServiceId, Service>("service")
            .with_table::<ProtocolParams, u128>("parameter")
            .with_table::<NodeIndex, Vec<ReportedReputationMeasurements>>("rep_measurements")
            .with_table::<NodeIndex, u8>("rep_scores")
            .with_table::<NodeIndex, u8>("submitted_rep_measurements")
            .with_table::<NodeIndex, NodeServed>("current_epoch_served")
            .with_table::<NodeIndex, NodeServed>("last_epoch_served")
            .with_table::<Epoch, TotalServed>("total_served")
            .with_table::<CommodityTypes, HpUfixed<6>>("commodity_prices")
            .with_table::<ServiceId, ServiceRevenue>("service_revenue")
            .with_table::<TxHash, ()>("executed_digests")
            .with_table::<NodeIndex, u8>("uptime")
            .with_table::<Blake3Hash, BTreeSet<NodeIndex>>("uri_to_node")
            .with_table::<NodeIndex, BTreeSet<Blake3Hash>>("node_to_uri")
            .enable_iter("current_epoch_served")
            .enable_iter("rep_measurements")
            .enable_iter("submitted_rep_measurements")
            .enable_iter("rep_scores")
            .enable_iter("latencies")
            .enable_iter("node")
            .enable_iter("executed_digests")
            .enable_iter("uptime")
            .enable_iter("service_revenue")
            .enable_iter("uri_to_node")
            .enable_iter("node_to_uri");

        #[cfg(debug_assertions)]
        {
            builder = builder
                .enable_iter("consensus_key_to_index")
                .enable_iter("pub_key_to_index");
        }

        // TODO(snormore): Move this to StateBuilder.
        StateTreeBuilder::new(builder).register_tables().into()
    }
}
