use lightning_application::Application;
use lightning_blockstore::blockstore::Blockstore;
use lightning_broadcast::Broadcast;
use lightning_interfaces::partial;
use lightning_notifier::Notifier;
use lightning_pool::PoolProvider;
use lightning_rep_collector::ReputationAggregator;
use lightning_rpc::Rpc;
use lightning_signer::Signer;
use lightning_topology::Topology;
use lightning_utils::config::TomlConfigProvider;

use crate::consensus::{MockConsensus, MockForwarder};
use crate::keys::EphemeralKeystore;

partial!(TestNodeComponents {
    ApplicationInterface = Application<Self>;
    BroadcastInterface = Broadcast<Self>;
    BlockstoreInterface = Blockstore<Self>;
    ConfigProviderInterface = TomlConfigProvider<Self>;
    // TODO(snormore): Support real consensus and forwarder, with configurable use of the mocks.
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
