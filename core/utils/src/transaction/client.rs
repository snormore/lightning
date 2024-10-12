use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{
    ExecuteTransactionOptions,
    ExecuteTransactionRetry,
    ExecuteTransactionWait,
    TransactionRequest,
    TransactionResponse,
    UpdateMethod,
};
use types::{ExecuteTransactionError, ExecuteTransactionResponse};

use super::{TransactionBuilder, TransactionSigner};
use crate::application::QueryRunnerExt;

// Maximum number of times we will resend a transaction.
const MAX_RETRIES: u8 = 3;

/// A client for submitting and executing transactions, and waiting for their receipts.
///
/// The client is signer-specific, and will sign the incoming transaction with the configured signer
/// before submitting it.
pub struct TransactionClient<C: NodeComponents> {
    app_query: c!(C::ApplicationInterface::SyncExecutor),
    notifier: C::NotifierInterface,
    mempool: MempoolSocket,
    signer: TransactionSigner,
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
        Self {
            app_query,
            notifier,
            mempool,
            signer,
            next_nonce,
        }
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
    ) -> Result<ExecuteTransactionResponse, ExecuteTransactionError> {
        let mut options = options.unwrap_or_default();

        // Default to retrying `MAX_RETRIES` times if not specified.
        match &options.retry {
            ExecuteTransactionRetry::Never => {},
            ExecuteTransactionRetry::Always(max_retries) => {
                if max_retries.is_none() {
                    options.retry = ExecuteTransactionRetry::Always(Some(MAX_RETRIES));
                }
            },
            ExecuteTransactionRetry::AlwaysExcept((max_retries, errors)) => {
                if max_retries.is_none() {
                    options.retry =
                        ExecuteTransactionRetry::AlwaysExcept((Some(MAX_RETRIES), errors.clone()));
                }
            },
            ExecuteTransactionRetry::OnlyWith((max_retries, ref errors)) => {
                if max_retries.is_none() {
                    options.retry =
                        ExecuteTransactionRetry::OnlyWith((Some(MAX_RETRIES), errors.clone()));
                }
            },
        }

        // TODO(snormore): Should we default the wait timeout to something if it's not specified?

        // TODO(snormore): Clean up this spawn/loop.

        // Spawn a tokio task to wait for the transaction receipt, retry if reverted, and return the
        // result containing the transaction request and receipt, or an error.
        let app_query = self.app_query.clone();
        let notifier = self.notifier.clone();
        let next_nonce = self.next_nonce.clone();
        let signer = self.signer.clone();
        let mempool = self.mempool.clone();
        let waiter_handle = spawn!(
            async move {
                let mut retry = 0;

                loop {
                    // Build and sign the transaction.
                    let chain_id = app_query.get_chain_id();
                    let next_nonce = next_nonce.fetch_add(1, Ordering::SeqCst);
                    let tx: TransactionRequest = TransactionBuilder::from_update(
                        method.clone(),
                        chain_id,
                        next_nonce,
                        &signer,
                    )
                    .into();

                    // Subscribe to executed blocks notifications before we enqueue the transaction.
                    let mut block_sub = notifier.subscribe_block_executed();

                    // Send transaction to the mempool.
                    // TODO(snormore): Simulate before we enqueue the transaction, returning error
                    // if it fails.
                    mempool.enqueue(tx.clone()).await.map_err(|e| {
                        ExecuteTransactionError::FailedToSubmitTransactionToMempool((
                            tx.clone(),
                            e.to_string(),
                        ))
                    })?;

                    // Wait for the transaction to be executed and get the receipt.
                    let receipt = async {
                        loop {
                            let Some(notification) = block_sub.recv().await else {
                                tracing::debug!("block subscription stream ended");
                                // TODO(snormore): Handle this better.
                                return Err(ExecuteTransactionError::Other(
                                    "block subscription stream ended".to_string(),
                                ));
                            };

                            for receipt in notification.response.txn_receipts {
                                if receipt.transaction_hash == tx.hash() {
                                    return Ok(receipt);
                                }
                            }
                        }
                    }
                    .await?;

                    match &receipt.response {
                        // If the transaction was successful, return the receipt.
                        TransactionResponse::Success(_) => {
                            return Ok(ExecuteTransactionResponse::Receipt((tx, receipt)));
                        },

                        // If the transaction was reverted, and retry is enabled for this type of
                        // revert, sleep for a short period and retry the transaction.
                        TransactionResponse::Revert(error) => {
                            match options.retry {
                                ExecuteTransactionRetry::OnlyWith((max_retries, ref errors)) => {
                                    if let Some(errors) = errors {
                                        if errors.contains(error) {
                                            retry += 1;

                                            if let Some(max_retries) = max_retries {
                                                if retry > max_retries {
                                                    tracing::warn!(
                                                        "transaction reverted and max retries reached (attempts: {}): {:?}",
                                                        retry,
                                                        receipt
                                                    );
                                                    return Err(ExecuteTransactionError::Reverted(
                                                        (tx, receipt),
                                                    ));
                                                }
                                            }

                                            tracing::info!(
                                                "retrying reverted transaction (hash: {:?}, response: {:?}, attempt: {}): {:?}",
                                                tx.hash(),
                                                receipt.response,
                                                retry + 1,
                                                tx
                                            );
                                            // TODO(snormore): Should we sleep/delay here for a bit?
                                        }
                                    }
                                },
                                ExecuteTransactionRetry::AlwaysExcept((
                                    max_retries,
                                    ref errors,
                                )) => {
                                    // If the error is in the exclude list, don't retry.
                                    if let Some(errors) = errors {
                                        if errors.contains(error) {
                                            tracing::warn!("transaction reverted: {:?}", receipt);
                                            return Err(ExecuteTransactionError::Reverted((
                                                tx, receipt,
                                            )));
                                        }
                                    }

                                    // If we are within the retry limit, retry the transaction, or
                                    // return reverted if we've hit the limit.
                                    retry += 1;
                                    if let Some(max_retries) = max_retries {
                                        if retry > max_retries {
                                            tracing::warn!(
                                                "transaction reverted and max retries reached (attempts: {}): {:?}",
                                                retry,
                                                receipt
                                            );
                                            return Err(ExecuteTransactionError::Reverted((
                                                tx, receipt,
                                            )));
                                        }
                                    }

                                    // Otherwise, continue retrying.
                                    tracing::info!(
                                        "retrying reverted transaction (hash: {:?}, response: {:?}, attempt: {}): {:?}",
                                        tx.hash(),
                                        receipt.response,
                                        retry + 1,
                                        tx
                                    );
                                    // TODO(snormore): Should we sleep/delay here for a bit?
                                },
                                ExecuteTransactionRetry::Always(max_retries) => {
                                    // If we are within the retry limit, retry the transaction, or
                                    // return reverted if we've hit the limit.
                                    retry += 1;
                                    if let Some(max_retries) = max_retries {
                                        if retry > max_retries {
                                            tracing::warn!(
                                                "transaction reverted and max retries reached (attempts: {}): {:?}",
                                                retry,
                                                receipt
                                            );
                                            return Err(ExecuteTransactionError::Reverted((
                                                tx, receipt,
                                            )));
                                        }
                                    }

                                    // Otherwise, continue retrying.
                                    tracing::info!(
                                        "retrying reverted transaction (hash: {:?}, response: {:?}, attempt: {}): {:?}",
                                        tx.hash(),
                                        receipt.response,
                                        retry + 1,
                                        tx
                                    );
                                    // TODO(snormore): Should we sleep/delay here for a bit?
                                },
                                ExecuteTransactionRetry::Never => {
                                    tracing::warn!("transaction reverted: {:?}", receipt);
                                    return Err(ExecuteTransactionError::Reverted((tx, receipt)));
                                },
                            }
                        },
                    }
                }
            },
            "TRANSACTION-CLIENT: waiter"
        );

        // If we're not waiting for a receipt, spawn a tokio task to wait for the receipt, retry
        // if reverted and/or timeout if configured to do so, and then return success.
        if let ExecuteTransactionWait::None = options.wait {
            return Ok(ExecuteTransactionResponse::None);
        }

        // Otherwise, wait for the tokio task to complete and return the result.
        let resp = waiter_handle.await??;
        tracing::debug!("transaction executed: {:?}", resp);
        Ok(resp)
    }
}
