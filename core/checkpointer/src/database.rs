use std::collections::HashSet;

use anyhow::Result;
use lightning_interfaces::types::Epoch;

use crate::config::CheckpointerDatabaseConfig;
use crate::headers::{AggregateCheckpointHeader, CheckpointHeader};

/// A trait for a checkpointer database, encapsulating the database operations that the
/// checkpointer needs to perform.
///
/// These operations are intentionally specific to uses within the checkpointer. They should
/// encapsulate any consistency needs internally to the implementation.
///
/// It is expected that implementations are thread-safe and can be shared between multiple
/// threads.
pub trait CheckpointerDatabase: Clone + Send + Sync {
    /// Build a new database instance using the given configuration.
    fn build(config: CheckpointerDatabaseConfig) -> Self;

    /// Get the set of checkpoint headers for the given epoch.
    fn get_checkpoint_headers(&self, epoch: Epoch) -> Result<HashSet<CheckpointHeader>>;

    /// Add a checkpoint header to the set of headers for the given epoch.
    fn add_checkpoint_header(&self, epoch: Epoch, header: CheckpointHeader) -> Result<()>;

    /// Get the aggregate checkpoint header for the given epoch.
    fn get_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
    ) -> Result<Option<AggregateCheckpointHeader>>;

    /// Set the aggregate checkpoint header for the given epoch.
    ///
    /// There is just a single one of these per epoch, and any existing entry for the epoch will
    /// be overwritten.
    fn set_aggregate_checkpoint_header(
        &self,
        epoch: Epoch,
        header: AggregateCheckpointHeader,
    ) -> Result<()>;
}
