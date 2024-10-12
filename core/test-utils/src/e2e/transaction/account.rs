use anyhow::Result;
use fleek_crypto::NodePublicKey;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    ExecuteTransactionError,
    ExecuteTransactionOptions,
    ExecuteTransactionResponse,
    ExecuteTransactionWait,
    ProofOfConsensus,
    Tokens,
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
    ) -> Result<ExecuteTransactionResponse, ExecuteTransactionError> {
        self.inner.execute_transaction(method, options).await
    }

    async fn execute_transaction_and_wait_for_receipt(
        &self,
        method: UpdateMethod,
        options: Option<ExecuteTransactionOptions>,
    ) -> Result<ExecuteTransactionResponse, ExecuteTransactionError> {
        let mut options = options.unwrap_or_default();

        if let ExecuteTransactionWait::None = options.wait {
            options.wait = ExecuteTransactionWait::Receipt(None);
        }

        self.execute_transaction(method, Some(options)).await
    }

    async fn deposit_and_stake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError> {
        // Deposit FLK tokens.
        self.execute_transaction_and_wait_for_receipt(
            UpdateMethod::Deposit {
                proof: ProofOfConsensus {},
                token: Tokens::FLK,
                amount: amount.clone(),
            },
            None,
        )
        .await?;

        // Stake FLK tokens.
        self.execute_transaction_and_wait_for_receipt(
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
        self.execute_transaction_and_wait_for_receipt(
            UpdateMethod::StakeLock { node, locked_for },
            None,
        )
        .await?;

        Ok(())
    }

    async fn unstake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), ExecuteTransactionError> {
        self.execute_transaction_and_wait_for_receipt(
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
