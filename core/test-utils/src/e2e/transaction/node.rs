use anyhow::Result;
use fleek_crypto::NodePublicKey;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    ExecuteTransactionError,
    ExecuteTransactionOptions,
    ExecuteTransactionRequest,
    ExecuteTransactionResponse,
    ProofOfConsensus,
    Tokens,
    TransactionReceipt,
    TransactionRequest,
    UpdateMethod,
};
use lightning_interfaces::SignerSubmitTxSocket;

use crate::e2e::NetworkTransactionClient;

pub struct NodeTransactionClient {
    signer: SignerSubmitTxSocket,
}

impl NodeTransactionClient {
    pub fn new(signer: SignerSubmitTxSocket) -> Self {
        Self { signer }
    }
}

#[async_trait::async_trait]
impl NetworkTransactionClient for NodeTransactionClient {
    async fn execute_transaction(
        &self,
        method: UpdateMethod,
        options: Option<ExecuteTransactionOptions>,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        let resp = self
            .signer
            .run(ExecuteTransactionRequest { method, options })
            .await??;

        match resp {
            ExecuteTransactionResponse::Receipt((request, receipt)) => Ok((request, receipt)),
            _ => unreachable!("invalid response from signer"),
        }
    }

    // TODO(snormore): Does this really need to exist on the node tx client?

    async fn deposit_and_stake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError> {
        // Deposit FLK tokens.
        self.execute_transaction(
            UpdateMethod::Deposit {
                proof: ProofOfConsensus {},
                token: Tokens::FLK,
                amount: amount.clone(),
            },
            None,
        )
        .await?;

        // Stake FLK tokens.
        self.execute_transaction(
            UpdateMethod::Stake {
                amount: amount.clone(),
                node_public_key: node,
                consensus_key: Default::default(),
                node_domain: None,
                worker_public_key: None,
                worker_domain: None,
                ports: None,
            },
            None,
        )
        .await?;

        Ok(())
    }

    async fn stake_lock(
        &self,
        locked_for: u64,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError> {
        self.execute_transaction(UpdateMethod::StakeLock { node, locked_for }, None)
            .await?;

        Ok(())
    }

    async fn unstake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError> {
        self.execute_transaction(
            UpdateMethod::Unstake {
                amount: amount.clone(),
                node,
            },
            None,
        )
        .await?;

        Ok(())
    }
}
