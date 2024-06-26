use std::time::Duration;

use lightning_utils::config::LIGHTNING_HOME_DIR;
use resolved_pathbuf::ResolvedPathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Interval for submitting the aggregated DACKs to the mempool
    pub submit_interval: Duration,
    /// Path to the database where the DACKs are stored
    pub db_path: ResolvedPathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            submit_interval: Duration::from_secs(10),
            db_path: LIGHTNING_HOME_DIR
                .join("data/dack_aggregator")
                .try_into()
                .expect("Failed to resolve path"),
        }
    }
}
