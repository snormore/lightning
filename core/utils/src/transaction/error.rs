use lightning_interfaces::types::{TransactionReceipt, TransactionRequest};
use thiserror::Error;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ApplicationClientError {
    // TODO(snormore): Clean this up
    #[error("transaction not successful: {:?}", .0.0.hash())]
    NotSuccess((TransactionRequest, TransactionReceipt)),

    #[error("transaction timeout retrying: {:?}", .0.hash())]
    TimeoutRetrying(TransactionRequest),

    #[error("transaction timeout waiting for receipt: {:?}", .0.hash())]
    TimeoutWaitingForReceipt(TransactionRequest),

    #[error("transaction failed to send to mempool: {0:?}")]
    Mempool(mpsc::error::SendError<TransactionRequest>),

    #[error("internal error: {0}")]
    Internal(String),
}
