use lightning_interfaces::types::{TransactionReceipt, TransactionRequest};
use thiserror::Error;
use tokio::sync::mpsc;

/// The transaction client error enum encapsulates errors that can occur when executing transactions
/// with the [`TransactionClient`].
#[derive(Debug, Error, Eq, PartialEq)]
pub enum TransactionClientError {
    // The transaction was submitted but reverted during execution for the reason in the receipt.
    #[error("Transaction was reverted: {:?}", .0.0.hash())]
    Reverted((TransactionRequest, TransactionReceipt)),

    /// The transaction exceeded timeout that encompasses the whole execution process including
    /// waiting and retries.
    #[error("Transaction timeout: {:?}", .0.hash())]
    Timeout(TransactionRequest),

    /// The transaction failed to send to mempool.
    #[error("transaction failed to send to mempool: {0:?}")]
    MempoolSendFailed(
        (
            TransactionRequest,
            mpsc::error::SendError<TransactionRequest>,
        ),
    ),

    /// An internal or unknown error occurred.
    #[error("Internal: {:?}", .0)]
    Internal((TransactionRequest, String)),
}
