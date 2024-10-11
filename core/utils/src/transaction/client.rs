use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{
    ExecuteTransactionOptions,
    ExecuteTransactionRetry,
    ExecuteTransactionWait,
    TransactionReceipt,
    TransactionRequest,
    TransactionResponse,
    UpdateMethod,
};
use tokio::sync::oneshot;
use types::{ExecuteTransactionError, ExecutionError};

use super::listener::TransactionReceiptListener;
use super::{TransactionBuilder, TransactionSigner};
use crate::application::QueryRunnerExt;

/// A client for submitting and executing transactions, and waiting for their receipts.
///
/// The client is signer-specific, and will sign the incoming transaction with the configured signer
/// before submitting it.
pub struct TransactionClient<C: NodeComponents> {
    app_query: c!(C::ApplicationInterface::SyncExecutor),
    mempool: MempoolSocket,
    signer: TransactionSigner,
    listener: TransactionReceiptListener<C>,
    next_nonce: Arc<AtomicU64>,
}

impl<C: NodeComponents> TransactionClient<C> {
    pub async fn new(
        app_query: c!(C::ApplicationInterface::SyncExecutor),
        notifier: C::NotifierInterface,
        mempool: MempoolSocket,
        signer: TransactionSigner,
        crucial: Option<ShutdownWaiter>,
    ) -> Self {
        let next_nonce = Arc::new(AtomicU64::new(signer.get_nonce(&app_query) + 1));
        let listener = TransactionReceiptListener::spawn(
            app_query.clone(),
            notifier.clone(),
            signer.clone(),
            next_nonce.clone(),
            crucial,
        )
        .await;

        Self {
            app_query,
            mempool,
            signer,
            listener,
            next_nonce,
        }
    }

    /// Submit an update request to the application executor and wait for it to be executed. Returns
    /// the transaction request and its receipt.
    pub async fn execute_transaction_with_retry_on_invalid_nonce(
        &self,
        method: UpdateMethod,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        self.execute_transaction(
            method,
            Some(ExecuteTransactionOptions {
                retry: ExecuteTransactionRetry::Errors((
                    HashSet::from_iter(vec![ExecutionError::InvalidNonce]),
                    None,
                )),
                ..Default::default()
            }),
        )
        .await
    }

    /// Submit an update request to the application executor and wait for it to be executed. Returns
    /// the transaction request and its receipt.
    ///
    /// This method also accepts options to configure the behavior of the transaction execution,
    /// such as the timeout for waiting for the transaction receipt, and the retry behavior for
    /// reverted transactions.
    ///
    /// If the transaction is not executed within a timeout, an error is returned.
    pub async fn execute_transaction(
        &self,
        method: UpdateMethod,
        options: Option<ExecuteTransactionOptions>,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        let options = options.unwrap_or_default();

        let chain_id = self.app_query.get_chain_id();
        let start = tokio::time::Instant::now();
        loop {
            // Build and sign the transaction.
            let next_nonce = self.next_nonce.fetch_add(1, Ordering::SeqCst);
            let tx: TransactionRequest =
                TransactionBuilder::from_update(method.clone(), chain_id, next_nonce, &self.signer)
                    .into();

            // If we've timed out, return an error.
            if let Some(timeout) = options.timeout {
                if start.elapsed() >= timeout {
                    return Err(ExecuteTransactionError::Timeout(tx));
                }
            }

            // Register transaction with pending transactions listener.
            let receipt_rx = self.listener.register(tx.hash()).await;

            // Send transaction to the mempool.
            self.mempool.enqueue(tx.clone()).await.map_err(|e| {
                ExecuteTransactionError::FailedToSubmitTransactionToMempool((
                    tx.clone(),
                    e.to_string(),
                ))
            })?;

            // Wait for the transaction to be executed, and return the receipt.
            // TODO(snormore): Support no timeout/wait here or before calling this?
            let receipt = self
                .wait_for_receipt(tx.clone(), receipt_rx, options.wait.clone())
                .await?;

            // If the transaction was reverted, and retry is enabled for this type of revert, sleep
            // for a short period and retry the transaction.
            if let TransactionResponse::Revert(error) = &receipt.response {
                match &options.retry {
                    ExecuteTransactionRetry::Errors((errors, _)) => {
                        // TODO(snormore): Implement max retries.
                        if errors.contains(error) {
                            tracing::info!(
                                "retrying reverted transaction (hash: {:?}, response: {:?}): {:?}",
                                tx.hash(),
                                receipt.response,
                                tx
                            );
                            // TODO(snormore): Make this configurable.
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    },
                    ExecuteTransactionRetry::AnyError(_) => {
                        // TODO(snormore): Implement max retries.
                    },
                    ExecuteTransactionRetry::Never => {},
                }
            }

            // If the transaction was reverted, return a reverted error.
            if let TransactionResponse::Revert(_) = receipt.response {
                return Err(ExecuteTransactionError::Reverted((tx, receipt)));
            }

            // Otherwise, return success with the receipt.
            return Ok((tx, receipt));
        }
    }

    /// Wait for a transaction receipt for a given transaction.
    ///
    /// If the transaction is not executed within a timeout, an error is returned.
    async fn wait_for_receipt(
        &self,
        tx: TransactionRequest,
        receipt_rx: oneshot::Receiver<TransactionReceipt>,
        options: ExecuteTransactionWait,
    ) -> Result<TransactionReceipt, ExecuteTransactionError> {
        // TODO(snormore): Support no timeout/wait here or before calling this?
        let timeout = match options {
            ExecuteTransactionWait::Receipt(timeout) => timeout,
            _ => None,
        }
        .unwrap_or(Duration::from_secs(10));
        let timeout_fut = tokio::time::sleep(timeout);
        tokio::pin!(timeout_fut);
        tokio::select! {
            result = receipt_rx => {
                match result {
                    Ok(receipt) => {
                        match receipt.response {
                            TransactionResponse::Success(_) => {
                                tracing::debug!("transaction executed: {:?}", receipt);
                            },
                            TransactionResponse::Revert(_) => {
                                tracing::debug!("transaction reverted: {:?}", receipt);
                            },
                        }
                        Ok(receipt)
                    },
                    Err(_) => {
                        // Handle when the oneshot channel is closed (sender dropped).
                        Err(ExecuteTransactionError::Other(
                            "transaction receipt channel closed".to_string(),
                        ))
                    }
                }
            },
            _ = &mut timeout_fut => {
                tracing::debug!("timeout while waiting for transaction receipt: {:?}", tx.hash());
                Err(ExecuteTransactionError::Timeout(tx))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use fleek_crypto::{AccountOwnerSecretKey, SecretKey};
    use lightning_application::{Application, ApplicationConfig};
    use lightning_interfaces::prelude::*;
    use lightning_node::Node;
    use lightning_notifier::Notifier;
    use lightning_test_utils::consensus::{
        Config as MockConsensusConfig,
        MockConsensus,
        MockForwarder,
    };
    use lightning_test_utils::json_config::JsonConfigProvider;
    use lightning_test_utils::keys::EphemeralKeystore;
    use tempfile::tempdir;
    use types::{ExecutionData, Genesis, GenesisAccount, GenesisNode, HandshakePorts, NodePorts};

    use super::*;

    partial_node_components!(TestNodeComponents {
        ConfigProviderInterface = JsonConfigProvider;
        ApplicationInterface = Application<Self>;
        NotifierInterface = Notifier<Self>;
        KeystoreInterface = EphemeralKeystore<Self>;
        ConsensusInterface = MockConsensus<Self>;
        ForwarderInterface = MockForwarder<Self>;
    });

    #[tokio::test]
    async fn test_execute_transaction_with_account_signer() {
        let temp_dir = tempdir().unwrap();

        let account_secret_key = AccountOwnerSecretKey::generate();
        let genesis = Genesis {
            account: vec![GenesisAccount {
                public_key: account_secret_key.to_pk().into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let genesis_path = genesis
            .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
            .unwrap();
        let mut node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default().with(
                JsonConfigProvider::default()
                    .with::<Application<TestNodeComponents>>(ApplicationConfig::test(genesis_path))
                    .with::<MockConsensus<TestNodeComponents>>(MockConsensusConfig {
                        min_ordering_time: 0,
                        max_ordering_time: 0,
                        probability_txn_lost: 0.0,
                        transactions_to_lose: HashSet::new(),
                        new_block_interval: Duration::from_secs(0),
                    }),
            ),
        )
        .unwrap();
        node.start().await;
        let app = node.provider.get::<Application<TestNodeComponents>>();
        let notifier = node.provider.get::<Notifier<TestNodeComponents>>();
        let forwarder = node.provider.get::<MockForwarder<TestNodeComponents>>();

        // Build a transaction client.
        let client = TransactionClient::<TestNodeComponents>::new(
            app.sync_query(),
            notifier.clone(),
            forwarder.mempool_socket(),
            TransactionSigner::AccountOwner(account_secret_key),
            None,
        )
        .await;

        // Execute a transaction and wait for it to complete.
        let (tx, receipt) = client
            .execute_transaction(UpdateMethod::IncrementNonce {}, None)
            .await
            .unwrap();
        assert_eq!(
            receipt.response,
            TransactionResponse::Success(ExecutionData::None)
        );
        assert!(!tx.hash().is_empty());
        assert_eq!(receipt.transaction_hash, tx.hash());

        // Shutdown the node.
        node.shutdown().await;
    }

    #[tokio::test]
    async fn test_execute_transaction_with_node_signer() {
        let temp_dir = tempdir().unwrap();
        let keystore = EphemeralKeystore::<TestNodeComponents>::default();
        let node_secret_key = keystore.get_ed25519_sk();
        let genesis = Genesis {
            node_info: vec![GenesisNode {
                owner: AccountOwnerSecretKey::generate().to_pk().into(),
                primary_public_key: node_secret_key.to_pk(),
                primary_domain: "127.0.0.1".parse().unwrap(),
                consensus_public_key: keystore.get_bls_pk(),
                worker_domain: "127.0.0.1".parse().unwrap(),
                worker_public_key: node_secret_key.to_pk(),
                ports: NodePorts {
                    primary: 0,
                    worker: 0,
                    mempool: 0,
                    rpc: 0,
                    pool: 0,
                    pinger: 0,
                    handshake: HandshakePorts {
                        http: 0,
                        webrtc: 0,
                        webtransport: 0,
                    },
                },
                stake: Default::default(),
                reputation: None,
                current_epoch_served: None,
                genesis_committee: true,
            }],
            ..Default::default()
        };
        let genesis_path = genesis
            .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
            .unwrap();
        let mut node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default().with(keystore).with(
                JsonConfigProvider::default()
                    .with::<Application<TestNodeComponents>>(ApplicationConfig::test(genesis_path))
                    .with::<MockConsensus<TestNodeComponents>>(MockConsensusConfig {
                        min_ordering_time: 0,
                        max_ordering_time: 0,
                        probability_txn_lost: 0.0,
                        transactions_to_lose: HashSet::new(),
                        new_block_interval: Duration::from_secs(0),
                    }),
            ),
        )
        .unwrap();
        node.start().await;
        let app = node.provider.get::<Application<TestNodeComponents>>();
        let notifier = node.provider.get::<Notifier<TestNodeComponents>>();
        let forwarder = node.provider.get::<MockForwarder<TestNodeComponents>>();

        // Build a transaction client.
        let client = TransactionClient::<TestNodeComponents>::new(
            app.sync_query(),
            notifier.clone(),
            forwarder.mempool_socket(),
            TransactionSigner::NodeMain(node_secret_key),
            None,
        )
        .await;

        // Execute a transaction and wait for it to complete.
        let (tx, receipt) = client
            .execute_transaction(UpdateMethod::IncrementNonce {}, None)
            .await
            .unwrap();
        assert_eq!(
            receipt.response,
            TransactionResponse::Success(ExecutionData::None)
        );
        assert!(!tx.hash().is_empty());
        assert_eq!(receipt.transaction_hash, tx.hash());

        // Shutdown the node.
        node.shutdown().await;
    }
}
