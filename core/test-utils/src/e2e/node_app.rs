use std::time::Duration;

use anyhow::Result;
use fleek_crypto::{AccountOwnerSecretKey, EthAddress, SecretKey};
use hp_fixed::unsigned::HpUfixed;
use lightning_interfaces::types::{
    Epoch,
    ExecutionError,
    Metadata,
    ProofOfConsensus,
    Tokens,
    TransactionReceipt,
    TransactionRequest,
    TransactionResponse,
    UpdateMethod,
    Value,
};
use lightning_interfaces::{
    ApplicationInterface,
    ForwarderInterface,
    KeystoreInterface,
    NotifierInterface,
    Subscriber,
    SyncQueryRunnerInterface,
};
use lightning_utils::application::QueryRunnerExt;
use thiserror::Error;
use tokio::sync::mpsc;

use super::{TestNode, TransactionSigner};
use crate::e2e::new_update_transaction;

#[derive(Debug, Error)]
pub enum ExecuteTransactionError {
    #[error("transaction not successful: {:?}", .0.0.hash())]
    NotSuccess((TransactionRequest, TransactionReceipt)),

    #[error("transaction timeout retrying: {:?}", .0.hash())]
    TimeoutRetrying(TransactionRequest),

    #[error("transaction timeout waiting for receipt: {:?}", .0.hash())]
    TimeoutWaitingForReceipt(TransactionRequest),

    #[error("transaction failed to send to mempool: {0:?}")]
    Mempool(mpsc::error::SendError<TransactionRequest>),

    #[error("internal error: {0}")]
    Internal(String),
}

impl TestNode {
    pub fn get_epoch(&self) -> Epoch {
        match self.app_query.get_metadata(&Metadata::Epoch) {
            Some(Value::Epoch(epoch)) => epoch,
            _ => unreachable!("invalid epoch in metadata"),
        }
    }

    pub fn get_protocol_fund_address(&self) -> EthAddress {
        match self.app_query.get_metadata(&Metadata::ProtocolFundAddress) {
            Some(Value::AccountPublicKey(s)) => s,
            None => unreachable!("missing protocol fund address in metadata"),
            _ => unreachable!("invalid protocol fund address in metadata"),
        }
    }

    pub fn get_total_supply(&self) -> HpUfixed<18> {
        match self.app_query.get_metadata(&Metadata::TotalSupply) {
            Some(Value::HpUfixed(s)) => s,
            None => panic!("missing total supply in metadata"),
            _ => unreachable!("invalid total supply in metadata"),
        }
    }

    pub fn get_supply_year_start(&self) -> HpUfixed<18> {
        match self.app_query.get_metadata(&Metadata::SupplyYearStart) {
            Some(Value::HpUfixed(s)) => s,
            None => panic!("missing supply year start in metadata"),
            _ => unreachable!("invalid supply year start in metadata"),
        }
    }

    pub fn get_stake(&self) -> HpUfixed<18> {
        self.app
            .sync_query()
            .get_node_info(&self.index(), |node| node.stake.staked)
            .ok_or(anyhow::anyhow!("own node not found"))
            .unwrap_or_default()
    }

    pub fn get_nonce(&self) -> u64 {
        self.app
            .sync_query()
            .get_node_info(&self.index(), |node| node.nonce)
            .ok_or(anyhow::anyhow!("own node not found"))
            .unwrap_or_default()
    }

    pub fn get_owner_nonce(&self) -> u64 {
        self.app_query
            .get_account_info(&self.owner_secret_key.to_pk().into(), |a| a.nonce)
            .unwrap_or_default()
    }

    pub fn get_account_nonce(&self, account: EthAddress) -> u64 {
        self.app_query
            .get_account_info(&account, |a| a.nonce)
            .unwrap_or_default()
    }

    pub fn get_stables_balance(&self, account: EthAddress) -> HpUfixed<6> {
        match self
            .app
            .sync_query()
            .get_account_info(&account, |a| a.stables_balance)
        {
            Some(balance) => balance,
            None => HpUfixed::<6>::zero(),
        }
    }

    pub fn get_flk_balance(&self, account: EthAddress) -> HpUfixed<18> {
        match self.app_query.get_account_info(&account, |a| a.flk_balance) {
            Some(balance) => balance,
            None => HpUfixed::<18>::zero(),
        }
    }

    pub async fn deposit_and_stake(
        &self,
        amount: HpUfixed<18>,
        account: &AccountOwnerSecretKey,
    ) -> Result<(), ExecuteTransactionError> {
        let address = account.to_pk().into();
        let signer = TransactionSigner::AccountOwner(account.clone());
        let nonce = self.get_account_nonce(address);

        // Deposit FLK tokens.
        self.execute_transaction(
            UpdateMethod::Deposit {
                proof: ProofOfConsensus {},
                token: Tokens::FLK,
                amount: amount.clone(),
            },
            signer.clone(),
            nonce + 1,
        )
        .await?;

        // Stake FLK tokens.
        self.execute_transaction(
            UpdateMethod::Stake {
                amount: amount.clone(),
                node_public_key: self.keystore.get_ed25519_pk(),
                consensus_key: Some(self.keystore.get_bls_pk()),
                node_domain: None,
                worker_public_key: None,
                worker_domain: None,
                ports: None,
            },
            signer,
            nonce + 2,
        )
        .await?;

        Ok(())
    }

    pub async fn stake_lock(
        &self,
        locked_for: u64,
        account: &AccountOwnerSecretKey,
    ) -> Result<(), ExecuteTransactionError> {
        let address = account.to_pk().into();
        let signer = TransactionSigner::AccountOwner(account.clone());
        let nonce = self.get_account_nonce(address);

        self.execute_transaction(
            UpdateMethod::StakeLock {
                node: self.keystore.get_ed25519_pk(),
                locked_for,
            },
            signer,
            nonce + 1,
        )
        .await?;

        Ok(())
    }

    pub async fn unstake(
        &self,
        amount: HpUfixed<18>,
        account: &AccountOwnerSecretKey,
    ) -> Result<(), ExecuteTransactionError> {
        let address = account.to_pk().into();
        let signer = TransactionSigner::AccountOwner(account.clone());
        let nonce = self.get_account_nonce(address);

        self.execute_transaction(
            UpdateMethod::Unstake {
                amount: amount.clone(),
                node: self.keystore.get_ed25519_pk(),
            },
            signer,
            nonce + 1,
        )
        .await?;

        Ok(())
    }

    pub async fn execute_transaction_from_node(
        &self,
        method: UpdateMethod,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        self.execute_transaction(
            method,
            TransactionSigner::NodeMain(self.keystore.get_ed25519_sk()),
            self.get_nonce() + 1,
        )
        .await
    }

    pub async fn execute_transaction(
        &self,
        method: UpdateMethod,
        signer: TransactionSigner,
        nonce: u64,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        let (tx, receipt) = self
            .execute_transaction_with_retry(method, false, signer, nonce)
            .await?;
        match receipt.response {
            TransactionResponse::Success(_) => Ok((tx, receipt)),
            _ => Err(ExecuteTransactionError::NotSuccess((tx, receipt))),
        }
    }

    pub async fn execution_transactions(
        &self,
        methods: Vec<UpdateMethod>,
        first_nonce: u64,
    ) -> Result<Vec<(TransactionRequest, TransactionReceipt)>, ExecuteTransactionError> {
        let mut transactions = Vec::new();
        for method in methods {
            let (tx, receipt) = self
                .execute_transaction(
                    method,
                    TransactionSigner::NodeMain(self.keystore.get_ed25519_sk()),
                    first_nonce,
                )
                .await?;
            transactions.push((tx, receipt));
        }
        Ok(transactions)
    }

    /// Submit an update request to the application executor and wait for it to be executed. Returns
    /// the transaction request and its receipt.
    ///
    /// If the transaction is not executed within a timeout, an error is returned.
    pub async fn execute_transaction_with_retry(
        &self,
        method: UpdateMethod,
        // TODO(snormore): Clean this up, can we do this without the bool arg? Also DRY it up with
        // what's in the committee beacon sub components.
        retry_invalid_nonce: bool,
        signer: TransactionSigner,
        nonce: u64,
    ) -> Result<(TransactionRequest, TransactionReceipt), ExecuteTransactionError> {
        let chain_id = self.app_query.get_chain_id();
        let timeout = Duration::from_secs(30);
        let start = tokio::time::Instant::now();
        loop {
            // Build and sign the transaction.
            let tx: TransactionRequest =
                new_update_transaction(method.clone(), chain_id, nonce, signer.clone()).into();

            // If we've timed out, return an error.
            if start.elapsed() >= timeout {
                return Err(ExecuteTransactionError::TimeoutRetrying(tx));
            }

            // Send transaction to the mempool.
            tracing::debug!(
                "sending transaction to mempool (nonce: {}): {:?}",
                nonce,
                tx.hash()
            );
            // TODO(snormore): We need to subscribe to blocks before sending the transaction or else
            // we may race it and not receive the notification.
            self.forwarder
                .mempool_socket()
                .enqueue(tx.clone())
                .await
                .map_err(ExecuteTransactionError::Mempool)?;

            // Wait for the transaction to be executed, and return the receipt.
            let receipt = self.wait_for_receipt(tx.clone()).await?;

            // Retry if the transaction was reverted because of invalid nonce.
            // This means our node sent multiple transactions asynchronously that landed in the same
            // block.
            if retry_invalid_nonce
                && receipt.response == TransactionResponse::Revert(ExecutionError::InvalidNonce)
            {
                tracing::warn!(
                    "transaction {:?} reverted due to invalid nonce, retrying",
                    tx.hash()
                );
                // TODO(snormore): Can we remove this whole retry on invalid nonce stuff?
                tokio::time::sleep(Duration::from_millis(200)).await;
                continue;
            }

            return Ok((tx, receipt));
        }
    }

    /// Wait for a transaction receipt for a given transaction.
    ///
    /// If the transaction is not executed within a timeout, an error is returned.
    async fn wait_for_receipt(
        &self,
        tx: TransactionRequest,
    ) -> Result<TransactionReceipt, ExecuteTransactionError> {
        // TODO(snormore): Consider using a shared subscription.
        let mut block_sub = self.notifier.subscribe_block_executed();
        // TODO(snormore): What should this timeout be?
        let timeout_fut = tokio::time::sleep(Duration::from_secs(30));
        tokio::pin!(timeout_fut);
        loop {
            tokio::select! {
                notification = block_sub.recv() => {
                    match notification {
                        Some(notification) => {
                            let response = notification.response;
                            for receipt in response.txn_receipts {
                                if receipt.transaction_hash == tx.hash() {
                                    match receipt.response {
                                        TransactionResponse::Success(_) => {
                                            tracing::debug!("transaction executed: {:?}", receipt);
                                        },
                                        TransactionResponse::Revert(_) => {
                                            // TODO(snormore): What to do here or in the caller/listener when transactions are reverted?
                                            tracing::warn!("transaction reverted: {:?}", receipt);
                                        },
                                    }
                                    return Ok(receipt);
                                }
                            }
                            continue;
                        },
                        None => {
                            // Notifier is not running, exit
                            return Err(ExecuteTransactionError::Internal("notifier is not running".to_string()));
                        }
                    }
                },
                _ = &mut timeout_fut => {
                    tracing::warn!("timeout while waiting for transaction receipt: {:?}", tx.hash());
                    return Err(ExecuteTransactionError::TimeoutWaitingForReceipt(tx));
                },
            }
        }
    }
}
