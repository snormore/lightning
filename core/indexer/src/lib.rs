#[cfg(test)]
mod tests;

use std::marker::PhantomData;
use std::sync::{Arc, OnceLock};

use fleek_crypto::NodePublicKey;
use lightning_interfaces::prelude::*;
use lightning_interfaces::types::{Blake3Hash, ContentUpdate, NodeIndex, UpdateMethod};
use lightning_interfaces::SubmitTxSocket;
pub struct Indexer<C: Collection> {
    pk: NodePublicKey,
    local_index: Arc<OnceLock<NodeIndex>>,
    submit_tx: SubmitTxSocket,
    query_runner: c![C::ApplicationInterface::SyncExecutor],
    _marker: PhantomData<C>,
}

impl<C: Collection> Clone for Indexer<C> {
    fn clone(&self) -> Self {
        Self {
            pk: self.pk,
            local_index: self.local_index.clone(),
            submit_tx: self.submit_tx.clone(),
            query_runner: self.query_runner.clone(),
            _marker: PhantomData,
        }
    }
}

impl<C: Collection> Indexer<C> {
    fn init(
        keystore: &C::KeystoreInterface,
        signer: &C::SignerInterface,
        fdi::Cloned(query_runner): fdi::Cloned<c!(C::ApplicationInterface::SyncExecutor)>,
    ) -> anyhow::Result<Self> {
        let pk = keystore.get_ed25519_pk();
        let local_index = OnceLock::new();
        if let Some(index) = query_runner.pubkey_to_index(&pk) {
            local_index.set(index).expect("Cell to be empty");
        }

        Ok(Self {
            pk,
            local_index: Arc::new(local_index),
            submit_tx: signer.get_socket(),
            query_runner,
            _marker: PhantomData,
        })
    }

    fn get_index(&self) -> Option<NodeIndex> {
        match self.local_index.get() {
            None => {
                if let Some(index) = self.query_runner.pubkey_to_index(&self.pk) {
                    let _ = self.local_index.set(index);
                    Some(index)
                } else {
                    None
                }
            },
            Some(index) => Some(*index),
        }
    }
}

impl<C: Collection> BuildGraph for Indexer<C> {
    fn build_graph() -> fdi::DependencyGraph {
        fdi::DependencyGraph::default().with(Self::init)
    }
}

impl<C: Collection> IndexerInterface<C> for Indexer<C> {
    async fn register(&self, uri: Blake3Hash) {
        if let Some(index) = self.get_index() {
            if self
                .query_runner
                .get_content_registry(&index)
                .map(|registry| !registry.contains(&uri))
                .unwrap_or(true)
            {
                let updates = vec![ContentUpdate { uri, remove: false }];
                if let Err(e) = self
                    .submit_tx
                    .enqueue(UpdateMethod::UpdateContentRegistry { updates })
                    .await
                {
                    tracing::error!("Submitting content registry update failed: {e:?}");
                }
            }
        }
    }

    async fn unregister(&self, uri: Blake3Hash) {
        if let Some(index) = self.get_index() {
            if self
                .query_runner
                .get_content_registry(&index)
                .map(|registry| registry.contains(&uri))
                .unwrap_or(false)
            {
                let updates = vec![ContentUpdate { uri, remove: true }];

                if let Err(e) = self
                    .submit_tx
                    .enqueue(UpdateMethod::UpdateContentRegistry { updates })
                    .await
                {
                    tracing::error!("Submitting content registry update failed: {e:?}");
                }
            }
        }
    }
}
