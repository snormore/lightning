use std::any::Any;
use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Result;
use fleek_crypto::{
    AccountOwnerPublicKey,
    AccountOwnerSecretKey,
    ConsensusPublicKey,
    ConsensusSecretKey,
    EthAddress,
    NodePublicKey,
    NodeSecretKey,
    SecretKey,
};
use lightning_application::Application;
use lightning_checkpointer::{CheckpointBroadcastMessage, Checkpointer, CheckpointerQuery};
use lightning_committee_beacon::{CommitteeBeaconComponent, CommitteeBeaconQuery};
use lightning_interfaces::prelude::*;
use lightning_interfaces::Events;
use lightning_node::ContainedNode;
use lightning_pool::PoolProvider;
use lightning_rep_collector::MyReputationReporter;
use lightning_rpc::{load_hmac_secret, Rpc, RpcClient};
use lightning_signer::Signer;
use lightning_utils::transaction::{TransactionClient, TransactionSigner};
use merklize::StateRootHash;
use ready::tokio::TokioReadyWaiter;
use ready::ReadyWaiter;
use types::{CheckpointAttestation, Epoch, Genesis, NodeIndex, NodeInfo, Topic};

use super::{
    AccountTransactionClient,
    NetworkNode,
    NetworkQueryRunner,
    NetworkTransactionClient,
    NodeTransactionClient,
    TestQueryRunner,
};
use crate::consensus::MockForwarder;
use crate::keys::EphemeralKeystore;

pub struct TestNode<C: NodeComponents> {
    pub inner: ContainedNode<C>,
    pub before_genesis_ready: TokioReadyWaiter<TestNodeBeforeGenesisReadyState>,
    pub after_genesis_ready: TokioReadyWaiter<()>,
    pub home_dir: PathBuf,
    pub owner_secret_key: AccountOwnerSecretKey,
    pub app: fdi::Ref<Application<C>>,
    pub app_query: c!(C::ApplicationInterface::SyncExecutor),
    pub broadcast: fdi::Ref<C::BroadcastInterface>,
    pub checkpointer: fdi::Ref<Checkpointer<C>>,
    pub committee_beacon: fdi::Ref<CommitteeBeaconComponent<C>>,
    pub notifier: fdi::Ref<C::NotifierInterface>,
    pub forwarder: fdi::Ref<MockForwarder<C>>,
    pub keystore: fdi::Ref<EphemeralKeystore<C>>,
    pub pool: fdi::Ref<PoolProvider<C>>,
    pub rpc: fdi::Ref<Rpc<C>>,
    pub reputation_reporter: fdi::Ref<MyReputationReporter>,
    pub signer: fdi::Ref<Signer<C>>,
}

#[async_trait::async_trait]
impl<C: NodeComponents> NetworkNode for TestNode<C> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn index(&self) -> NodeIndex {
        self.app
            .sync_query()
            .pubkey_to_index(&self.keystore.get_ed25519_pk())
            .expect("failed to get node index")
    }

    async fn shutdown(self: Box<Self>) {
        self.inner.shutdown().await;
    }

    fn get_node_secret_key(&self) -> NodeSecretKey {
        self.keystore.get_ed25519_sk()
    }

    fn get_node_public_key(&self) -> NodePublicKey {
        self.keystore.get_ed25519_pk()
    }

    fn get_consensus_secret_key(&self) -> ConsensusSecretKey {
        self.keystore.get_bls_sk()
    }

    fn get_consensus_public_key(&self) -> ConsensusPublicKey {
        self.keystore.get_bls_pk()
    }

    fn get_owner_secret_key(&self) -> AccountOwnerSecretKey {
        self.owner_secret_key.clone()
    }

    fn get_owner_public_key(&self) -> AccountOwnerPublicKey {
        self.owner_secret_key.to_pk()
    }

    async fn apply_genesis(&self, genesis: Genesis) -> Result<bool> {
        self.app.apply_genesis(genesis).await
    }

    async fn wait_for_before_genesis_ready(&self) {
        self.before_genesis_ready.wait().await;
    }

    fn get_before_genesis_ready_state(&self) -> Option<TestNodeBeforeGenesisReadyState> {
        self.before_genesis_ready.state().clone()
    }

    async fn wait_for_after_genesis_ready(&self) {
        self.after_genesis_ready.wait().await;
    }

    async fn get_pool_connected_peers(&self) -> Result<Vec<NodeIndex>> {
        self.pool.connected_peers().await
    }

    fn emit_epoch_changed_notification(
        &self,
        epoch: Epoch,
        previous_state_root: StateRootHash,
        new_state_root: StateRootHash,
        last_epoch_hash: [u8; 32],
    ) {
        self.notifier.get_emitter().epoch_changed(
            epoch,
            last_epoch_hash,
            previous_state_root,
            new_state_root,
        );
    }

    async fn broadcast_checkpoint_attestation(&self, header: CheckpointAttestation) -> Result<()> {
        self.broadcast
            .get_pubsub::<CheckpointBroadcastMessage>(Topic::Checkpoint)
            .send(
                &CheckpointBroadcastMessage::CheckpointAttestation(header),
                None,
            )
            .await?;

        Ok(())
    }

    async fn node_transaction_client(&self) -> Box<dyn NetworkTransactionClient> {
        Box::new(NodeTransactionClient::new(self.signer.get_socket()))
    }

    async fn transaction_client(
        &self,
        signer: TransactionSigner,
    ) -> Box<dyn NetworkTransactionClient> {
        Box::new(AccountTransactionClient::new(
            TransactionClient::<C>::new(
                self.app_query.clone(),
                self.notifier.clone(),
                self.forwarder.mempool_socket(),
                signer,
                None,
            )
            .await,
        ))
    }

    fn application_query(&self) -> Box<dyn NetworkQueryRunner> {
        Box::new(TestQueryRunner::new(self.app_query.clone()))
    }

    fn checkpointer_query(&self) -> CheckpointerQuery {
        self.checkpointer.query()
    }

    fn committee_beacon_query(&self) -> CommitteeBeaconQuery {
        self.committee_beacon.query()
    }

    fn reputation_reporter(&self) -> MyReputationReporter {
        self.reputation_reporter.clone()
    }

    fn rpc_client(&self) -> Result<RpcClient> {
        let addr = self.rpc.listen_address().expect("rpc not ready");
        RpcClient::new_no_auth(&format!("http://{}", addr))
    }

    async fn rpc_admin_client(&self) -> Result<RpcClient> {
        let secret = load_hmac_secret(Some(self.home_dir.clone()))?;
        let addr = self.rpc.listen_address().expect("rpc not ready");
        RpcClient::new(&format!("http://{}/admin", addr), Some(&secret)).await
    }

    async fn rpc_ws_client(&self) -> Result<jsonrpsee::ws_client::WsClient> {
        let addr = self.rpc.listen_address().expect("rpc not ready");
        jsonrpsee::ws_client::WsClientBuilder::default()
            .build(&format!("ws://{}", addr))
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    fn rpc_event_tx(&self) -> Events {
        self.rpc.event_tx()
    }
}

impl<C: NodeComponents> TestNode<C> {
    pub fn get_node_info(&self) -> Option<NodeInfo> {
        self.app_query.get_node_info(&self.index(), |n| n)
    }

    pub fn get_consensus_secret_key(&self) -> ConsensusSecretKey {
        self.keystore.get_bls_sk()
    }

    pub fn get_node_secret_key(&self) -> NodeSecretKey {
        self.keystore.get_ed25519_sk()
    }

    pub fn get_owner_address(&self) -> EthAddress {
        self.owner_secret_key.to_pk().into()
    }

    pub fn get_node_signer(&self) -> TransactionSigner {
        TransactionSigner::NodeMain(self.keystore.get_ed25519_sk())
    }

    pub fn get_owner_signer(&self) -> TransactionSigner {
        TransactionSigner::AccountOwner(self.owner_secret_key.clone())
    }
}

#[derive(Clone, Debug)]
pub struct TestNodeBeforeGenesisReadyState {
    pub pool_listen_address: SocketAddr,
    pub rpc_listen_address: SocketAddr,
}

impl Default for TestNodeBeforeGenesisReadyState {
    fn default() -> Self {
        Self {
            pool_listen_address: "0.0.0.0:0".parse().unwrap(),
            rpc_listen_address: "0.0.0.0:0".parse().unwrap(),
        }
    }
}
