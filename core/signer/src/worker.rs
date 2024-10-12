use std::sync::Arc;

use affair::AsyncWorker;
use lightning_interfaces::prelude::*;
use lightning_interfaces::SignerError;
use lightning_utils::transaction::TransactionClient;
use tokio::sync::Mutex;
use types::{ExecuteTransactionRequest, ExecuteTransactionResponse};

#[derive(Clone)]
pub struct SignerWorker<C: NodeComponents> {
    client: Arc<Mutex<Option<TransactionClient<C>>>>,
}

impl<C: NodeComponents> SignerWorker<C> {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&self, client: TransactionClient<C>) {
        // Lock mutex to update client within existing Arc/Mutex.
        // This ensures all clones of self.client see the update, as replacing the Arc would
        // disconnect existing clones.
        let mut client_lock = self.client.lock().await;
        *client_lock = Some(client);
    }
}

impl<C: NodeComponents> AsyncWorker for SignerWorker<C> {
    type Request = ExecuteTransactionRequest;
    type Response = Result<ExecuteTransactionResponse, SignerError>;

    async fn handle(&mut self, request: Self::Request) -> Self::Response {
        tracing::debug!("handling signer request: {:?}", request);

        // Check that the client is ready, and return an error if not.
        let client = self.client.lock().await;
        let Some(client) = client.as_ref() else {
            tracing::warn!("signer not ready");
            return Err(SignerError::NotReady);
        };

        // Execute the transaction via the client.
        let resp = client
            .execute_transaction(request.method, request.options)
            .await?;

        Ok(resp)
    }
}
