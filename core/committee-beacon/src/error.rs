use lightning_interfaces::types::{ExecuteTransactionError, Value};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommitteeBeaconError {
    /// The committee beacon has an unknown phase type.
    #[error("committee beacon unknown phase type: {0:?}")]
    UnknownPhaseType(Option<Value>),

    /// The own node was not found.
    #[error("own node not found")]
    OwnNodeNotFound,

    /// The transaction execution failed.
    #[error(transparent)]
    ExecuteTransaction(#[from] ExecuteTransactionError),

    /// Other generic error.
    #[error("committee beacon error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for CommitteeBeaconError {
    fn from(error: anyhow::Error) -> Self {
        CommitteeBeaconError::Other(error.to_string())
    }
}

impl From<String> for CommitteeBeaconError {
    fn from(error: String) -> Self {
        CommitteeBeaconError::Other(error)
    }
}
