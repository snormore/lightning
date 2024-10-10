use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{
    TransactionReceipt,
    TransactionRequest,
    TransactionResponse,
    UpdateMethod,
};
use tokio::sync::oneshot;
use types::ExecutionError;

use super::listener::TransactionReceiptListener;
use super::{TransactionBuilder, TransactionClientError, TransactionSigner};
use crate::application::QueryRunnerExt;

#[derive(Debug, Clone)]
pub struct ExecuteTransactionOptions {
    pub wait_for_receipt_timeout: Duration,
    pub retry_on_revert: HashSet<TransactionResponse>,
    pub retry_on_revert_delay: Duration,
    pub execution_timeout: Duration,
}

impl Default for ExecuteTransactionOptions {
    fn default() -> Self {
        Self {
            wait_for_receipt_timeout: Duration::from_secs(30),
            retry_on_revert: HashSet::new(),
            retry_on_revert_delay: Duration::from_millis(100),
            execution_timeout: Duration::from_secs(30),
        }
    }
}

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
    ) -> Self {
        let next_nonce = Arc::new(AtomicU64::new(signer.get_nonce(&app_query) + 1));
        let listener = TransactionReceiptListener::spawn(
            app_query.clone(),
            notifier.clone(),
            signer.clone(),
            next_nonce.clone(),
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
    ///
    /// If the transaction is not executed within a timeout, an error is returned.
    pub async fn execute_transaction(
        &self,
        method: UpdateMethod,
    ) -> Result<(TransactionRequest, TransactionReceipt), TransactionClientError> {
        let (tx, receipt) = self
            .execute_transaction_with_options(method, Default::default())
            .await?;
        match receipt.response {
            TransactionResponse::Success(_) => Ok((tx, receipt)),
            TransactionResponse::Revert(_) => Err(TransactionClientError::Reverted((tx, receipt))),
        }
    }

    /// Submit an update request to the application executor and wait for it to be executed. Returns
    /// the transaction request and its receipt.
    pub async fn execute_transaction_with_retry_on_invalid_nonce(
        &self,
        method: UpdateMethod,
    ) -> Result<(TransactionRequest, TransactionReceipt), TransactionClientError> {
        self.execute_transaction_with_options(
            method,
            ExecuteTransactionOptions {
                retry_on_revert: HashSet::from_iter(vec![TransactionResponse::Revert(
                    ExecutionError::InvalidNonce,
                )]),
                ..Default::default()
            },
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
    pub async fn execute_transaction_with_options(
        &self,
        method: UpdateMethod,
        options: ExecuteTransactionOptions,
    ) -> Result<(TransactionRequest, TransactionReceipt), TransactionClientError> {
        let chain_id = self.app_query.get_chain_id();
        let start = tokio::time::Instant::now();
        loop {
            // Build and sign the transaction.
            let next_nonce = self.next_nonce.fetch_add(1, Ordering::SeqCst);
            let tx: TransactionRequest =
                TransactionBuilder::from_update(method.clone(), chain_id, next_nonce, &self.signer)
                    .into();

            // If we've timed out, return an error.
            if start.elapsed() >= options.execution_timeout {
                return Err(TransactionClientError::Timeout(tx));
            }

            // Register transaction with pending transactions listener.
            let receipt_rx = self.listener.register(tx.hash()).await;

            // Send transaction to the mempool.
            self.mempool
                .enqueue(tx.clone())
                .await
                .map_err(|e| TransactionClientError::MempoolSendFailed((tx.clone(), e)))?;

            // Wait for the transaction to be executed, and return the receipt.
            let receipt = self
                .wait_for_receipt(tx.clone(), receipt_rx, options.wait_for_receipt_timeout)
                .await?;

            // If the transaction was reverted, and retry is enabled for this type of revert, sleep
            // for a short period and retry the transaction.
            if options.retry_on_revert.contains(&receipt.response) {
                tracing::info!(
                    "retrying reverted transaction (hash: {:?}, response: {:?}): {:?}",
                    tx.hash(),
                    receipt.response,
                    tx
                );
                tokio::time::sleep(options.retry_on_revert_delay).await;
                continue;
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
        timeout: Duration,
    ) -> Result<TransactionReceipt, TransactionClientError> {
        let timeout_fut = tokio::time::sleep(timeout);
        tokio::pin!(timeout_fut);
        tokio::select! {
            result = receipt_rx => {
                let receipt = result.map_err(|e|
                    TransactionClientError::Internal((tx, e.to_string())))?;
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
            _ = &mut timeout_fut => {
                tracing::debug!("timeout while waiting for transaction receipt: {:?}", tx.hash());
                Err(TransactionClientError::Timeout(tx))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use fleek_crypto::{AccountOwnerSecretKey, NodeSecretKey, SecretKey};
    use lightning_application::config::StorageConfig;
    use lightning_application::{Application, ApplicationConfig};
    use lightning_forwarder::Forwarder;
    use lightning_interfaces::prelude::*;
    use lightning_keystore::Keystore;
    use lightning_node::Node;
    use lightning_notifier::Notifier;
    use lightning_test_utils::json_config::JsonConfigProvider;
    use lightning_test_utils::keys::EphemeralKeystore;
    use types::ExecutionData;

    use super::*;

    partial_node_components!(TestNodeComponents {
        ConfigProviderInterface = JsonConfigProvider;
        ApplicationInterface = Application<Self>;
        NotifierInterface = Notifier<Self>;
        ForwarderInterface = Forwarder<Self>;
        KeystoreInterface = Keystore<Self>;
    });

    #[tokio::test]
    async fn test_execute_transaction_with_account_signer() {
        let account_secret_key = AccountOwnerSecretKey::generate();
        let mut node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default()
                .with(EphemeralKeystore::<TestNodeComponents>::default())
                .with(
                    JsonConfigProvider::default().with::<Application<TestNodeComponents>>(
                        ApplicationConfig {
                            storage: StorageConfig::InMemory,
                            network: None,
                            genesis_path: None,
                            db_path: None,
                            db_options: None,
                            dev: None,
                        },
                    ),
                ),
        )
        .unwrap();
        let app = node.provider.get::<Application<TestNodeComponents>>();
        let notifier = node.provider.get::<Notifier<TestNodeComponents>>();
        let forwarder = node.provider.get::<Forwarder<TestNodeComponents>>();

        // Build a transaction client.
        let client = TransactionClient::<TestNodeComponents>::new(
            app.sync_query(),
            notifier.clone(),
            forwarder.mempool_socket(),
            TransactionSigner::AccountOwner(account_secret_key),
        )
        .await;

        // Execute a transaction and wait for it to complete.
        let (tx, receipt) = client
            .execute_transaction(UpdateMethod::IncrementNonce {})
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
        let node_secret_key = NodeSecretKey::generate();
        let mut node = Node::<TestNodeComponents>::init_with_provider(
            fdi::Provider::default()
                .with(EphemeralKeystore::<TestNodeComponents>::default())
                .with(
                    JsonConfigProvider::default().with::<Application<TestNodeComponents>>(
                        ApplicationConfig {
                            storage: StorageConfig::InMemory,
                            network: None,
                            genesis_path: None,
                            db_path: None,
                            db_options: None,
                            dev: None,
                        },
                    ),
                ),
        )
        .unwrap();
        let app = node.provider.get::<Application<TestNodeComponents>>();
        let notifier = node.provider.get::<Notifier<TestNodeComponents>>();
        let forwarder = node.provider.get::<Forwarder<TestNodeComponents>>();

        // Build a transaction client.
        let client = TransactionClient::<TestNodeComponents>::new(
            app.sync_query(),
            notifier.clone(),
            forwarder.mempool_socket(),
            TransactionSigner::NodeMain(node_secret_key),
        )
        .await;

        // Execute a transaction and wait for it to complete.
        let (tx, receipt) = client
            .execute_transaction(UpdateMethod::IncrementNonce {})
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
