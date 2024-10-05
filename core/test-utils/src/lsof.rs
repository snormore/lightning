use std::path::PathBuf;
use std::process::Command;

use lightning_utils::poll::{poll_until, PollUntilError};
use thiserror::Error;
use tokio::time::Duration;

/// Wait for the file to not be open by any process, using `lsof`.
pub async fn wait_for_file_to_close(
    path: &PathBuf,
    timeout: Duration,
    delay: Duration,
) -> Result<(), WaitForFileToCloseError> {
    if !path.exists() {
        return Ok(());
    }

    poll_until(
        || async {
            let status = Command::new("lsof")
                .arg(path.to_str().unwrap())
                .status()
                .map_err(WaitForFileToCloseError::CommandError)?;

            if status.code() == Some(1) {
                return Ok(());
            }

            tracing::warn!("file is still open: {:?}", path);
            // TODO(snormore): Remove this when finished debugging.
            println!("DEBUG: file is still open: {:?}", path);
            Err(PollUntilError::ConditionNotSatisfied)
        },
        timeout,
        delay,
    )
    .await?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum WaitForFileToCloseError {
    #[error("Timeout reached")]
    Timeout,

    #[error("Command error: {0:?}")]
    CommandError(std::io::Error),

    #[error("Internal error: {0:?}")]
    Internal(String),
}

impl From<PollUntilError> for WaitForFileToCloseError {
    fn from(error: PollUntilError) -> Self {
        match error {
            PollUntilError::Timeout => Self::Timeout,
            PollUntilError::ConditionError(e) => Self::Internal(e),
            PollUntilError::ConditionNotSatisfied => {
                unreachable!()
            },
        }
    }
}

impl From<WaitForFileToCloseError> for PollUntilError {
    fn from(error: WaitForFileToCloseError) -> Self {
        match error {
            WaitForFileToCloseError::Timeout => Self::Timeout,
            WaitForFileToCloseError::CommandError(e) => Self::ConditionError(e.to_string()),
            WaitForFileToCloseError::Internal(e) => Self::ConditionError(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn test_wait_for_file_to_close() {
        let temp_dir = tempdir().unwrap();
        let path = temp_dir.path().join("test.txt");
        let file = File::create(&path).unwrap();

        // Check that the file is open.
        let result =
            wait_for_file_to_close(&path, Duration::from_millis(200), Duration::from_millis(50))
                .await;
        assert!(matches!(
            result.unwrap_err(),
            WaitForFileToCloseError::Timeout
        ));

        // Drop and close the file.
        drop(file);

        // Check that it's now open.
        wait_for_file_to_close(&path, Duration::from_secs(1), Duration::from_millis(50))
            .await
            .unwrap();
    }
}
