use anyhow::Result;
use fleek_crypto::NodePublicKey;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    ExecuteTransactionError,
    ExecuteTransactionOptions,
    ProofOfConsensus,
    Tokens,
    TransactionReceipt,
    TransactionRequest,
    UpdateMethod,
};
use lightning_interfaces::NodeComponents;
use lightning_utils::transaction::TransactionClient;

use crate::e2e::NetworkTransactionClient;

pub struct AccountTransactionClient<C: NodeComponents> {
    inner: TransactionClient<C>,
}

impl<C: NodeComponents> AccountTransactionClient<C> {
    pub fn new(inner: TransactionClient<C>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<C: NodeComponents> NetworkTransactionClient for AccountTransactionClient<C> {
    async fn execute_transaction(
        &self,
        method: UpdateMethod,
        options: Option<ExecuteTransactionOptions>,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        self.inner.execute_transaction(method, options).await
    }

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
