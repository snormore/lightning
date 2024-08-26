use std::collections::BTreeSet;
use std::time::Duration;

use anyhow::Result;
use atomo::{
    Atomo,
    KeyIterator,
    QueryPerm,
    ResolvedTableReference,
    SerdeBackend,
    StorageBackend,
    StorageBackendConstructor,
    TableSelector,
};
use fleek_crypto::{ClientPublicKey, EthAddress, NodePublicKey};
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
    TransactionRequest,
    TransactionResponse,
    TxHash,
    Value,
};
use lightning_interfaces::SyncQueryRunnerInterface;
use lightning_types::{StateProofKey, StateProofValue};
use merklize::{StateRootHash, StateTree};

use crate::state::ApplicationState;

#[derive(Clone)]
pub struct QueryRunner<T: StateTree> {
    db: Atomo<QueryPerm, <T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
    tree: T,

    metadata_table: ResolvedTableReference<Metadata, Value>,
    account_table: ResolvedTableReference<EthAddress, AccountInfo>,
    client_table: ResolvedTableReference<ClientPublicKey, EthAddress>,
    node_table: ResolvedTableReference<NodeIndex, NodeInfo>,
    pub_key_to_index: ResolvedTableReference<NodePublicKey, NodeIndex>,
    committee_table: ResolvedTableReference<Epoch, Committee>,
    services_table: ResolvedTableReference<ServiceId, Service>,
    param_table: ResolvedTableReference<ProtocolParams, u128>,
    current_epoch_served: ResolvedTableReference<NodeIndex, NodeServed>,
    rep_measurements: ResolvedTableReference<NodeIndex, Vec<ReportedReputationMeasurements>>,
    latencies: ResolvedTableReference<(NodeIndex, NodeIndex), Duration>,
    rep_scores: ResolvedTableReference<NodeIndex, u8>,
    _last_epoch_served: ResolvedTableReference<NodeIndex, NodeServed>,
    total_served_table: ResolvedTableReference<Epoch, TotalServed>,
    _service_revenue: ResolvedTableReference<ServiceId, ServiceRevenue>,
    _commodity_price: ResolvedTableReference<CommodityTypes, HpUfixed<6>>,
    executed_digests_table: ResolvedTableReference<TxHash, ()>,
    uptime_table: ResolvedTableReference<NodeIndex, u8>,
    uri_to_node: ResolvedTableReference<Blake3Hash, BTreeSet<NodeIndex>>,
    node_to_uri: ResolvedTableReference<NodeIndex, BTreeSet<Blake3Hash>>,
}

impl<T: StateTree> QueryRunner<T> {
    pub fn run<F, R>(&self, query: F) -> R
    where
        F: FnOnce(
            &mut TableSelector<<T::StorageBuilder as StorageBackendConstructor>::Storage, T::Serde>,
        ) -> R,
    {
        self.db.run(query)
    }
}

impl<T: StateTree> SyncQueryRunnerInterface for QueryRunner<T>
where
    T: StateTree + Send + Sync + Clone + 'static,
    T::StorageBuilder: StorageBackendConstructor + Send + Sync + Clone,
    <T::StorageBuilder as StorageBackendConstructor>::Storage: StorageBackend + Send + Sync + Clone,
    T::Serde: SerdeBackend + Send + Sync + Clone,
    // TODO(snormore): Can we DRY this up, should the bounds just be on the StateTree trait?
{
    type StorageBuilder = T::StorageBuilder;
    type Serde = T::Serde;
    type StateTree = T;

    fn new(
        db: Atomo<
            QueryPerm,
            <T::StorageBuilder as StorageBackendConstructor>::Storage,
            Self::Serde,
        >,
        tree: Self::StateTree,
    ) -> Self {
        Self {
            metadata_table: db.resolve::<Metadata, Value>("metadata"),
            account_table: db.resolve::<EthAddress, AccountInfo>("account"),
            client_table: db.resolve::<ClientPublicKey, EthAddress>("client_keys"),
            node_table: db.resolve::<NodeIndex, NodeInfo>("node"),
            pub_key_to_index: db.resolve::<NodePublicKey, NodeIndex>("pub_key_to_index"),
            committee_table: db.resolve::<Epoch, Committee>("committee"),
            services_table: db.resolve::<ServiceId, Service>("service"),
            param_table: db.resolve::<ProtocolParams, u128>("parameter"),
            current_epoch_served: db.resolve::<NodeIndex, NodeServed>("current_epoch_served"),
            rep_measurements: db
                .resolve::<NodeIndex, Vec<ReportedReputationMeasurements>>("rep_measurements"),
            latencies: db.resolve::<(NodeIndex, NodeIndex), Duration>("latencies"),
            rep_scores: db.resolve::<NodeIndex, u8>("rep_scores"),
            _last_epoch_served: db.resolve::<NodeIndex, NodeServed>("last_epoch_served"),
            total_served_table: db.resolve::<Epoch, TotalServed>("total_served"),
            _commodity_price: db.resolve::<CommodityTypes, HpUfixed<6>>("commodity_prices"),
            _service_revenue: db.resolve::<ServiceId, ServiceRevenue>("service_revenue"),
            executed_digests_table: db.resolve::<TxHash, ()>("executed_digests"),
            uptime_table: db.resolve::<NodeIndex, u8>("uptime"),
            uri_to_node: db.resolve::<Blake3Hash, BTreeSet<NodeIndex>>("uri_to_node"),
            node_to_uri: db.resolve::<NodeIndex, BTreeSet<Blake3Hash>>("node_to_uri"),

            db,
            tree,
        }
    }

    // TODO(snormore): Remove this or fix it.
    // fn atomo_from_checkpoint(
    //     path: impl AsRef<Path>,
    //     hash: [u8; 32],
    //     checkpoint: &[u8],
    // ) -> anyhow::Result<
    //     Atomo<QueryPerm, <Self::StorageBuilder as StorageBackendConstructor>::Storage,
    // Self::Serde>,
    // > { let backend = AtomoStorageBuilder::new(Some(path.as_ref())) .from_checkpoint(hash,
    // > checkpoint) .read_only();

    //     let atomo = ApplicationState::register_tables(
    //         AtomoBuilder::<T::StorageBuilder, T::Serde>::new(backend),
    //     )
    //     .build()?
    //     .query();

    //     Ok(atomo)
    // }

    // fn atomo_from_path(
    //     path: impl AsRef<Path>,
    // ) -> anyhow::Result<
    //     Atomo<QueryPerm, <Self::StorageBuilder as StorageBackendConstructor>::Storage,
    // Self::Serde>,
    // > { let backend = AtomoStorageBuilder::new(Some(path.as_ref())).read_only();

    //     let atomo = ApplicationState::register_tables(
    //         AtomoBuilder::<T::StorageBuilder, T::Serde>::new(backend),
    //     )
    //     .build()?
    //     .query();

    //     Ok(atomo)
    // }

    fn get_metadata(&self, key: &Metadata) -> Option<Value> {
        self.db.run(|ctx| self.metadata_table.get(ctx).get(key))
    }

    /// Returns the state tree root hash from the application state.
    #[inline]
    fn get_state_root(&self) -> Result<StateRootHash> {
        self.run(|ctx| self.tree.get_state_root(ctx))
    }

    /// Returns the state proof for a given key from the application state, using the state tree.
    #[inline]
    fn get_state_proof(&self, key: StateProofKey) -> Result<(Option<StateProofValue>, T::Proof)> {
        self.run(|ctx| {
            let (table, serialized_key) = key.raw::<T::Serde>();
            let proof = self
                .tree
                .get_state_proof(ctx, &table, serialized_key.clone())?;
            let value = self
                .run(|ctx| ctx.get_raw_value(table, &serialized_key))
                .map(|value| key.value::<T::Serde>(value));
            Ok((value, proof))
        })
    }

    /// Verify the state tree.
    #[inline]
    // TODO(snormore): Can we make this not need a mut self?
    fn verify_state_tree(&mut self) -> Result<()> {
        self.tree.verify_state_tree_unsafe(&mut self.db)
    }

    /// Check if the state tree is empty.
    #[inline]
    // TODO(snormore): Can we make this not need a mut self?
    fn is_empty_state_tree(&mut self) -> Result<bool> {
        self.tree.is_empty_state_tree_unsafe(&mut self.db)
    }

    #[inline]
    fn get_account_info<V>(
        &self,
        address: &EthAddress,
        selector: impl FnOnce(AccountInfo) -> V,
    ) -> Option<V> {
        self.db
            .run(|ctx| self.account_table.get(ctx).get(address))
            .map(selector)
    }

    fn client_key_to_account_key(&self, pub_key: &ClientPublicKey) -> Option<EthAddress> {
        self.db.run(|ctx| self.client_table.get(ctx).get(pub_key))
    }

    #[inline]
    fn get_node_info<V>(
        &self,
        node: &NodeIndex,
        selector: impl FnOnce(NodeInfo) -> V,
    ) -> Option<V> {
        self.db
            .run(|ctx| self.node_table.get(ctx).get(node))
            .map(selector)
    }

    #[inline]
    fn get_node_table_iter<V>(&self, closure: impl FnOnce(KeyIterator<NodeIndex>) -> V) -> V {
        self.db.run(|ctx| closure(self.node_table.get(ctx).keys()))
    }

    fn pubkey_to_index(&self, pub_key: &NodePublicKey) -> Option<NodeIndex> {
        self.db
            .run(|ctx| self.pub_key_to_index.get(ctx).get(pub_key))
    }

    #[inline]
    fn get_committe_info<V>(
        &self,
        epoch: &Epoch,
        selector: impl FnOnce(Committee) -> V,
    ) -> Option<V> {
        self.db
            .run(|ctx| self.committee_table.get(ctx).get(epoch))
            .map(selector)
    }

    fn get_service_info(&self, id: &ServiceId) -> Option<Service> {
        self.db.run(|ctx| self.services_table.get(ctx).get(id))
    }

    fn get_protocol_param(&self, param: &ProtocolParams) -> Option<u128> {
        self.db.run(|ctx| self.param_table.get(ctx).get(param))
    }

    fn get_current_epoch_served(&self, node: &NodeIndex) -> Option<NodeServed> {
        self.db
            .run(|ctx| self.current_epoch_served.get(ctx).get(node))
    }

    fn get_reputation_measurements(
        &self,
        node: &NodeIndex,
    ) -> Option<Vec<ReportedReputationMeasurements>> {
        self.db.run(|ctx| self.rep_measurements.get(ctx).get(node))
    }

    fn get_latencies(&self, nodes: &(NodeIndex, NodeIndex)) -> Option<Duration> {
        self.db.run(|ctx| self.latencies.get(ctx).get(nodes))
    }

    fn get_latencies_iter<V>(
        &self,
        closure: impl FnOnce(KeyIterator<(NodeIndex, NodeIndex)>) -> V,
    ) -> V {
        self.db.run(|ctx| closure(self.latencies.get(ctx).keys()))
    }

    fn get_reputation_score(&self, node: &NodeIndex) -> Option<u8> {
        self.db.run(|ctx| self.rep_scores.get(ctx).get(node))
    }

    fn get_total_served(&self, epoch: &Epoch) -> Option<TotalServed> {
        self.db
            .run(|ctx| self.total_served_table.get(ctx).get(epoch))
    }

    fn has_executed_digest(&self, digest: [u8; 32]) -> bool {
        self.db
            .run(|ctx| self.executed_digests_table.get(ctx).get(digest))
            .is_some()
    }

    fn index_to_pubkey(&self, node_index: &NodeIndex) -> Option<NodePublicKey> {
        self.get_node_info::<NodePublicKey>(node_index, |node_info| node_info.public_key)
    }

    fn simulate_txn(&self, txn: TransactionRequest) -> TransactionResponse {
        self.db.run(|ctx| {
            let app = ApplicationState::<T>::executor(ctx);
            app.execute_transaction(txn)
        })
    }

    fn get_node_uptime(&self, node_index: &NodeIndex) -> Option<u8> {
        self.db
            .run(|ctx| self.uptime_table.get(ctx).get(node_index))
    }

    fn get_uri_providers(&self, uri: &Blake3Hash) -> Option<BTreeSet<NodeIndex>> {
        self.db.run(|ctx| self.uri_to_node.get(ctx).get(uri))
    }

    fn get_content_registry(&self, node_index: &NodeIndex) -> Option<BTreeSet<Blake3Hash>> {
        self.db.run(|ctx| self.node_to_uri.get(ctx).get(node_index))
    }
}
