use anyhow::Result;
use fleek_crypto::NodePublicKey;
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    ProofOfConsensus,
    Tokens,
    TransactionReceipt,
    TransactionRequest,
    UpdateMethod,
};
use lightning_interfaces::NodeComponents;
use lightning_utils::transaction::{TransactionClient, TransactionClientError};

use super::NetworkTransactionClient;

pub struct TestTransactionClient<C: NodeComponents> {
    inner: TransactionClient<C>,
}

impl<C: NodeComponents> TestTransactionClient<C> {
    pub fn new(inner: TransactionClient<C>) -> Self {
        Self { inner }
    }
}

#[async_trait::async_trait]
impl<C: NodeComponents> NetworkTransactionClient for TestTransactionClient<C> {
    async fn execute_transaction(
        &self,
        method: UpdateMethod,
    ) -> Result<(TransactionRequest, TransactionReceipt), TransactionClientError> {
        self.inner.execute_transaction(method).await
    }

    async fn deposit_and_stake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), TransactionClientError> {
        // Deposit FLK tokens.
        self.execute_transaction(UpdateMethod::Deposit {
            proof: ProofOfConsensus {},
            token: Tokens::FLK,
            amount: amount.clone(),
        })
        .await?;

        // Stake FLK tokens.
        self.execute_transaction(UpdateMethod::Stake {
            amount: amount.clone(),
            node_public_key: node,
            consensus_key: Default::default(),
            node_domain: None,
            worker_public_key: None,
            worker_domain: None,
            ports: None,
        })
        .await?;

        Ok(())
    }

    async fn stake_lock(
        &self,
        locked_for: u64,
        node: NodePublicKey,
    ) -> Result<(), TransactionClientError> {
        self.execute_transaction(UpdateMethod::StakeLock { node, locked_for })
            .await?;

        Ok(())
    }

    async fn unstake(
        &self,
        amount: HpUfixed<18>,
        node: NodePublicKey,
    ) -> Result<(), TransactionClientError> {
        self.execute_transaction(UpdateMethod::Unstake {
            amount: amount.clone(),
            node,
        })
        .await?;

        Ok(())
    }
}
