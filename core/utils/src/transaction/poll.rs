use std::future::Future;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PollUntilError {
    #[error("Condition error: {0:?}")]
    ConditionError(String),

    #[error("Condition not satisfied")]
    ConditionNotSatisfied,

    #[error("Timeout reached")]
    Timeout,
}

/// Polls asynchronously until the given condition is met, or a timeout is reached.
///
/// Returns `PollUntilError::Timeout` if the timeout is reached.
pub async fn poll_until<F, Fut, R>(
    condition: F,
    timeout: tokio::time::Duration,
    delay: tokio::time::Duration,
) -> Result<R, PollUntilError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<R, PollUntilError>>,
{
    let start = tokio::time::Instant::now();

    while start.elapsed() < timeout {
        match condition().await {
            Ok(result) => return Ok(result),
            Err(PollUntilError::ConditionNotSatisfied) => {
                tokio::time::sleep(delay).await;
                continue;
            },
            Err(e) => return Err(e),
        }
    }

    Err(PollUntilError::Timeout)
}

// TODO(snormore): Tests.
