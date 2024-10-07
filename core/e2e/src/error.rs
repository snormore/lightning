use lightning_node::NodeError;
use lightning_utils::poll::PollUntilError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SwarmError {
    #[error("Timeout waiting for swarm to be ready")]
    WaitForReadyTimeout,

    #[error("Internal: {0}")]
    Internal(String),
}

impl From<NodeError> for SwarmError {
    fn from(error: NodeError) -> Self {
        match error {
            NodeError::WaitForReadyTimeout(_) => SwarmError::WaitForReadyTimeout,
            NodeError::Internal(e) => SwarmError::Internal(e),
        }
    }
}

impl From<PollUntilError> for SwarmError {
    fn from(error: PollUntilError) -> Self {
        match error {
            PollUntilError::Timeout => SwarmError::WaitForReadyTimeout,
            PollUntilError::ConditionError(_) => SwarmError::Internal(error.to_string()),
            PollUntilError::ConditionNotSatisfied => {
                unreachable!("poll_until should never return ConditionNotSatisfied")
            },
        }
    }
}
