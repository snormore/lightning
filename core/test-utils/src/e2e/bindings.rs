use lightning_application::Application;
use lightning_blockstore::blockstore::Blockstore;
use lightning_broadcast::Broadcast;
use lightning_checkpointer::Checkpointer;
use lightning_committee_beacon::CommitteeBeaconComponent;
use lightning_interfaces::partial_node_components;
use lightning_notifier::Notifier;
use lightning_pool::PoolProvider;
use lightning_rep_collector::ReputationAggregator;
use lightning_rpc::Rpc;
use lightning_signer::Signer;
use lightning_topology::Topology;
use lightning_utils::config::TomlConfigProvider;
use tokio::sync::Mutex;

use crate::consensus::{MockConsensus, MockForwarder};
use crate::keys::EphemeralKeystore;

partial_node_components!(TestNodeComponents {
    ApplicationInterface = Application<Self>;
    BroadcastInterface = Broadcast<Self>;
    BlockstoreInterface = Blockstore<Self>;
    CheckpointerInterface = Checkpointer<Self>;
    CommitteeBeaconInterface = CommitteeBeaconComponent<Self>;
    ConfigProviderInterface = TomlConfigProvider<Self>;
    ConsensusInterface = MockConsensus<Self>;
    ForwarderInterface = MockForwarder<Self>;
    KeystoreInterface = EphemeralKeystore<Self>;
    NotifierInterface = Notifier<Self>;
    PoolInterface = PoolProvider<Self>;
    ReputationAggregatorInterface = ReputationAggregator<Self>;
    RpcInterface = Rpc<Self>;
    SignerInterface = Signer<Self>;
    TopologyInterface = Topology<Self>;
});

pub struct SyncWrapper<T> {
    inner: Mutex<T>,
}

unsafe impl<T> Sync for SyncWrapper<T> {}

impl<T> SyncWrapper<T> {
    pub fn new(data: T) -> Self {
        Self {
            inner: Mutex::new(data),
        }
    }

    // Provide methods to access or manipulate `inner`
    pub async fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut data = self.inner.lock().await;
        f(&mut *data)
    }
}
