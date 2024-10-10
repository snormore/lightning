use std::collections::{BTreeSet, HashMap};

use fleek_crypto::EthAddress;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    Blake3Hash,
    ChainId,
    CommitteeSelectionBeaconCommit,
    CommitteeSelectionBeaconPhase,
    CommitteeSelectionBeaconReveal,
    Epoch,
    EpochInfo,
    Metadata,
    NodeIndex,
    NodeInfo,
    ProtocolParamKey,
    ProtocolParamValue,
    ReportedReputationMeasurements,
    Service,
    ServiceId,
    Value,
};
use lightning_interfaces::SyncQueryRunnerInterface;
use lightning_utils::application::QueryRunnerExt;

use super::NetworkQueryRunner;

pub struct TestQueryRunner<Q: SyncQueryRunnerInterface> {
    inner: Q,
}

impl<Q: SyncQueryRunnerInterface> TestQueryRunner<Q> {
    pub fn new(inner: Q) -> Self {
        Self { inner }
    }
}

impl<Q: SyncQueryRunnerInterface> NetworkQueryRunner for TestQueryRunner<Q> {
    fn get_chain_id(&self) -> ChainId {
        self.inner.get_chain_id()
    }

    fn get_epoch(&self) -> Epoch {
        self.inner.get_current_epoch()
    }

    fn get_epoch_info(&self) -> EpochInfo {
        self.inner.get_epoch_info()
    }

    fn get_node_info(&self, node: NodeIndex) -> Option<NodeInfo> {
        self.inner.get_node_info(&node, |node| node)
    }

    fn get_metadata(&self, metadata: &Metadata) -> Option<Value> {
        self.inner.get_metadata(metadata)
    }

    fn get_protocol_param(&self, key: &ProtocolParamKey) -> Option<ProtocolParamValue> {
        self.inner.get_protocol_param(key)
    }

    fn get_committee_members(&self, epoch: Epoch) -> Option<Vec<NodeIndex>> {
        self.inner
            .get_committee_info(&epoch, |committee| committee.members)
    }

    fn get_protocol_fund_address(&self) -> EthAddress {
        match self.inner.get_metadata(&Metadata::ProtocolFundAddress) {
            Some(Value::AccountPublicKey(s)) => s,
            None => unreachable!("missing protocol fund address in metadata"),
            _ => unreachable!("invalid protocol fund address in metadata"),
        }
    }

    fn get_total_supply(&self) -> HpUfixed<18> {
        match self.inner.get_metadata(&Metadata::TotalSupply) {
            Some(Value::HpUfixed(s)) => s,
            None => panic!("missing total supply in metadata"),
            _ => unreachable!("invalid total supply in metadata"),
        }
    }

    fn get_supply_year_start(&self) -> HpUfixed<18> {
        match self.inner.get_metadata(&Metadata::SupplyYearStart) {
            Some(Value::HpUfixed(s)) => s,
            None => panic!("missing supply year start in metadata"),
            _ => unreachable!("invalid supply year start in metadata"),
        }
    }

    fn get_stake(&self, node: NodeIndex) -> HpUfixed<18> {
        self.inner
            .get_node_info(&node, |node| node.stake.staked)
            .ok_or(anyhow::anyhow!("own node not found"))
            .unwrap_or_default()
    }

    fn get_node_nonce(&self, node: NodeIndex) -> u64 {
        self.inner
            .get_node_info(&node, |node| node.nonce)
            .ok_or(anyhow::anyhow!("own node not found"))
            .unwrap_or_default()
    }

    fn get_account_nonce(&self, account: EthAddress) -> u64 {
        self.inner
            .get_account_info(&account, |a| a.nonce)
            .unwrap_or_default()
    }

    fn get_stables_balance(&self, account: EthAddress) -> HpUfixed<6> {
        match self.inner.get_account_info(&account, |a| a.stables_balance) {
            Some(balance) => balance,
            None => HpUfixed::<6>::zero(),
        }
    }

    fn get_flk_balance(&self, account: EthAddress) -> HpUfixed<18> {
        match self.inner.get_account_info(&account, |a| a.flk_balance) {
            Some(balance) => balance,
            None => HpUfixed::<18>::zero(),
        }
    }

    fn get_staking_amount(&self) -> u64 {
        self.inner.get_staking_amount()
    }

    fn get_service_info(&self, service: &ServiceId) -> Option<Service> {
        self.inner.get_service_info(service)
    }

    fn get_uri_providers(&self, uri: &Blake3Hash) -> Option<BTreeSet<NodeIndex>> {
        self.inner.get_uri_providers(uri)
    }

    fn get_content_registry(&self, node_index: &NodeIndex) -> Option<BTreeSet<Blake3Hash>> {
        self.inner.get_content_registry(node_index)
    }

    fn get_reputation_score(&self, node: &NodeIndex) -> Option<u8> {
        self.inner.get_reputation_score(node)
    }

    fn get_committee_selection_beacon_phase(&self) -> Option<CommitteeSelectionBeaconPhase> {
        match self
            .inner
            .get_metadata(&Metadata::CommitteeSelectionBeaconPhase)
        {
            Some(Value::CommitteeSelectionBeaconPhase(phase)) => Some(phase),
            None => None,
            _ => unreachable!("invalid committee selection beacon phase in metadata"),
        }
    }

    fn get_committee_selection_beacons(
        &self,
    ) -> HashMap<
        NodeIndex,
        (
            CommitteeSelectionBeaconCommit,
            Option<CommitteeSelectionBeaconReveal>,
        ),
    > {
        self.inner.get_committee_selection_beacons()
    }

    fn get_reputation_measurements(
        &self,
        node: &NodeIndex,
    ) -> Option<Vec<ReportedReputationMeasurements>> {
        self.inner.get_reputation_measurements(node)
    }
}
