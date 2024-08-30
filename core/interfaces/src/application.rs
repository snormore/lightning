use affair::Socket;
use anyhow::Result;
use fdi::BuildGraph;
use lightning_types::{ChainId, NodeInfo};

use crate::collection::Collection;
use crate::types::{Block, BlockExecutionResponse};
use crate::{ApplicationStateInterface, ConfigConsumer, SyncQueryRunnerInterface};

/// The socket that is handled by the application layer and fed by consensus (or other
/// synchronization systems in place) which executes and persists transactions that
/// are put into it.
///
/// # Safety
///
/// This socket should be used with as much caution as possible, for all intend and purposes
/// this socket should be sealed and preferably not accessible out side of the scope in which
/// it is created.
pub type ExecutionEngineSocket = Socket<Block, BlockExecutionResponse>;

#[interfaces_proc::blank]
pub trait ApplicationInterface<C: Collection>:
    BuildGraph + ConfigConsumer + Sized + Send + Sync
{
    /// The type for the sync query executor.
    type SyncExecutor: SyncQueryRunnerInterface;

    /// The type of the application state.
    type State: ApplicationStateInterface<C>;

    /// Returns the query runner for the application state.
    fn sync_query(&self) -> Self::SyncExecutor;

    /// Returns a socket that should be used to submit transactions to be executed
    /// by the application layer.
    ///
    /// # Safety
    ///
    /// See the safety document for the [`ExecutionEngineSocket`].
    #[socket]
    fn transaction_executor(&self) -> ExecutionEngineSocket;

    /// Will seed its underlying database with the checkpoint provided
    async fn load_from_checkpoint(
        config: &Self::Config,
        checkpoint: Vec<u8>,
        checkpoint_hash: [u8; 32],
    ) -> Result<()>;

    /// Used to get the chain id from the genesis file instead of state
    fn get_chain_id(config: &Self::Config) -> Result<ChainId>;

    /// Returns the committee from the geneis of the network
    fn get_genesis_committee(config: &Self::Config) -> Result<Vec<NodeInfo>>;

    /// Resets the state tree by clearing it and rebuilding it from the full state.
    ///
    /// This method is unsafe because it acts directly on the underlying storage backend.
    fn reset_state_tree_unsafe(config: &Self::Config) -> Result<()>;
}

#[derive(Clone, Debug)]
pub enum ExecutionError {
    InvalidSignature,
    InvalidNonce,
    InvalidProof,
    NotNodeOwner,
    NotCommitteeMember,
    NodeDoesNotExist,
    AlreadySignaled,
    NonExistingService,
}
