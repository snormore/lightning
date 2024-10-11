use std::collections::HashSet;
use std::time::Duration;

use thiserror::Error;

use crate::{ExecutionError, TransactionReceipt, TransactionRequest, UpdateMethod};

type MaxRetries = u32;

type Timeout = Duration;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExecuteTransactionRequest {
    pub method: UpdateMethod,
    pub options: Option<ExecuteTransactionOptions>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct ExecuteTransactionOptions {
    pub retry: ExecuteTransactionRetry,
    pub wait: ExecuteTransactionWait,
    pub timeout: Option<Timeout>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum ExecuteTransactionWait {
    #[default]
    None,
    Hash,
    Receipt(Option<Timeout>),
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum ExecuteTransactionResponse {
    #[default]
    None,
    Hash(TransactionRequest),
    Receipt((TransactionRequest, TransactionReceipt)),
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum ExecuteTransactionRetry {
    #[default]
    Never,
    AnyError(Option<MaxRetries>),
    Errors((HashSet<ExecutionError>, Option<MaxRetries>)),
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ExecuteTransactionError {
    // The transaction was submitted but reverted during execution for the reason in the receipt.
    #[error("Transaction was reverted: {:?}", .0.0.hash())]
    Reverted((TransactionRequest, TransactionReceipt)),

    /// The transaction execution timed out.
    #[error("Transaction timeout: {:?}", .0.hash())]
    Timeout(TransactionRequest),

    /// The transaction was not submitted to the signer.
    #[error("Failed to submit transaction to signer: {:?}", .0)]
    FailedToSubmitRequestToSigner(ExecuteTransactionRequest),

    /// The transaction failed to be submitted to the mempool.
    #[error("Failed to submit transaction to mempool: {:?}: {:?}", .0.0.hash(), .0.1)]
    FailedToSubmitTransactionToMempool((TransactionRequest, String)),

    /// Failed to get response from signer.
    #[error("Failed to get response from signer")]
    FailedToGetResponseFromSigner,

    /// The signer is not ready.
    #[error("Signer not ready")]
    SignerNotReady,

    /// Other generic error.
    #[error("Other: {:?}", .0)]
    Other(String),
}

impl From<affair::RunError<ExecuteTransactionRequest>> for ExecuteTransactionError {
    fn from(err: affair::RunError<ExecuteTransactionRequest>) -> Self {
        match err {
            affair::RunError::FailedToEnqueueReq(req) => {
                ExecuteTransactionError::FailedToSubmitRequestToSigner(req)
            },
            affair::RunError::FailedToGetResponse => {
                ExecuteTransactionError::FailedToGetResponseFromSigner
            },
        }
    }
}

impl From<anyhow::Error> for ExecuteTransactionError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(error.to_string())
    }
}

impl From<String> for ExecuteTransactionError {
    fn from(error: String) -> Self {
        Self::Other(error)
    }
}
