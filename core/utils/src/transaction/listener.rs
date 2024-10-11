use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{TransactionReceipt, TxHash};
use tokio::sync::{oneshot, Mutex};

use super::TransactionSigner;

/// The transaction receipt listener is responsible for listening for transaction receipts from the
/// network via the notifier executed blocks subscription, and sending them to the registered
/// channel.
pub(crate) struct TransactionReceiptListener<C: NodeComponents> {
    pending: Arc<Mutex<HashMap<TxHash, oneshot::Sender<TransactionReceipt>>>>,
    _components: PhantomData<C>,
}

impl<C: NodeComponents> TransactionReceiptListener<C> {
    pub fn new() -> Self {
        let pending = Arc::new(Mutex::new(HashMap::<
            TxHash,
            oneshot::Sender<TransactionReceipt>,
        >::new()));
        Self {
            pending: pending.clone(),
            _components: PhantomData,
        }
    }

    /// Register a new transaction hash that we should listen for and a channel to send the receipt.
    pub async fn register(&self, tx: TxHash) -> oneshot::Receiver<TransactionReceipt> {
        let (receipt_tx, receipt_rx) = oneshot::channel();
        self.pending.lock().await.insert(tx, receipt_tx);
        receipt_rx
    }

    /// Create and spawn a new transaction receipt listener, that's responsible for listening for
    /// transaction receipts from the network via the notifier executed blocks subscription, and
    /// sending them to the registered channel.
    ///
    /// This method is non-blocking and returns immediately after spawning the listener.
    ///
    /// The listener will run until the notifier subscription is closed, or the listener is
    /// explicitly shut down.
    pub async fn spawn(
        app_query: c!(C::ApplicationInterface::SyncExecutor),
        notifier: C::NotifierInterface,
        signer: TransactionSigner,
        next_nonce: Arc<AtomicU64>,
        crucial: Option<ShutdownWaiter>,
    ) -> Self {
        let listener = Self::new();
        let pending = listener.pending.clone();

        let fut = async move {
            let mut block_sub = notifier.subscribe_block_executed();
            loop {
                let Some(notification) = block_sub.recv().await else {
                    tracing::debug!("block subscription stream ended");
                    break;
                };

                // Update the next nonce counter from application state.
                next_nonce.store(signer.get_nonce(&app_query) + 1, Ordering::Relaxed);

                // Send pending receipts back through the registered channel.
                for receipt in notification.response.txn_receipts {
                    let mut pending = pending.lock().await;
                    if pending.contains_key(&receipt.transaction_hash) {
                        if let Some(sender) = pending.remove(&receipt.transaction_hash) {
                            let _ = sender.send(receipt);
                        }
                    }
                }
            }
        };

        if let Some(shutdown) = crucial {
            spawn!(fut, "TRANSACTION-CLIENT: listener", crucial(shutdown));
        } else {
            spawn!(fut, "TRANSACTION-CLIENT: listener");
        }

        listener
    }
}
