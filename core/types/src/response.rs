use ethers::types::{TransactionReceipt as EthersTxnReceipt, U64};
use fleek_crypto::TransactionSender;
use serde::{Deserialize, Serialize};

use super::{Epoch, NodeInfo};

/// Info on a Narwhal epoch
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Serialize, Deserialize)]
pub struct EpochInfo {
    /// List of committee members
    pub committee: Vec<NodeInfo>,
    /// The current epoch number
    pub epoch: Epoch,
    /// Timestamp when the epoch ends
    pub epoch_end: u64,
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Serialize, Deserialize)]
pub enum TransactionResponse {
    Success(ExecutionData),
    Revert(ExecutionError),
}

impl TransactionResponse {
    pub fn is_success(&self) -> bool {
        match self {
            Self::Success(_) => true,
            Self::Revert(_) => false,
        }
    }
}

// todo(dalton): Get something in here to indicate which function it called
#[derive(Clone, Debug, Hash, Serialize, Deserialize, Eq, PartialEq)]
pub struct TransactionReceipt {
    /// The hash of the block where the given transaction was included.
    pub block_hash: [u8; 32],
    /// The number of the block where the given transaction was included.
    pub block_number: u128,
    /// The index of the transaction within the block.
    pub transaction_index: u64,
    /// Hash of the transaction
    pub transaction_hash: [u8; 32],
    /// The sender of the transaction
    pub from: TransactionSender,
    /// The results of the transaction
    pub response: TransactionResponse,
}

impl From<TransactionReceipt> for EthersTxnReceipt {
    fn from(value: TransactionReceipt) -> Self {
        let sender = if let TransactionSender::AccountOwner(address) = value.from {
            address.0.into()
        } else {
            [0u8; 20].into()
        };
        Self {
            transaction_hash: value.transaction_hash.into(),
            transaction_index: value.transaction_index.into(),
            block_hash: Some(value.block_hash.into()),
            block_number: Some((value.block_number as u64).into()),
            from: sender,
            // todo
            to: None,
            status: Some(if value.response.is_success() {
                U64::one()
            } else {
                U64::zero()
            }),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Serialize, Deserialize)]
pub enum ExecutionData {
    None,
    String(String),
    UInt(u128),
    EpochInfo(EpochInfo),
    EpochChange,
}

/// Error type for transaction execution on the application layer
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash, Eq, Serialize, Deserialize)]
pub enum ExecutionError {
    InsufficientBalance,
    InvalidSignature,
    InvalidNonce,
    InvalidProof,
    InvalidInternetAddress,
    InsufficientNodeDetails,
    InvalidStateFunction,
    InvalidConsensusKey,
    InvalidToken,
    NoLockedTokens,
    TokensLocked,
    NotNodeOwner,
    NotCommitteeMember,
    NodeDoesNotExist,
    AlreadySignaled,
    NonExistingService,
    OnlyAccountOwner,
    OnlyNode,
    OnlyGovernance,
    InvalidServiceId,
    InsufficientStakesToLock,
    LockExceededMaxStakeLockTime,
    LockedTokensUnstakeForbidden,
    EpochAlreadyChanged,
    EpochHasNotStarted,
    ConsensusKeyAlreadyIndexed,
    Unimplemented,
}
