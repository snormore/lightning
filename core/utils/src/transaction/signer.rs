use fleek_crypto::{
    AccountOwnerSecretKey,
    NodeSecretKey,
    SecretKey,
    TransactionSender,
    TransactionSignature,
};
use lightning_interfaces::SyncQueryRunnerInterface;

#[derive(Clone)]
pub enum TransactionSigner {
    AccountOwner(AccountOwnerSecretKey),
    NodeMain(NodeSecretKey),
}

impl TransactionSigner {
    pub fn to_sender(&self) -> TransactionSender {
        match self {
            TransactionSigner::AccountOwner(sk) => {
                TransactionSender::AccountOwner(sk.to_pk().into())
            },
            TransactionSigner::NodeMain(sk) => TransactionSender::NodeMain(sk.to_pk()),
        }
    }

    pub fn sign(&self, digest: &[u8; 32]) -> TransactionSignature {
        match self {
            TransactionSigner::AccountOwner(sk) => {
                TransactionSignature::AccountOwner(sk.sign(digest))
            },
            TransactionSigner::NodeMain(sk) => TransactionSignature::NodeMain(sk.sign(digest)),
        }
    }

    pub fn get_nonce<Q: SyncQueryRunnerInterface>(&self, app_query: &Q) -> u64 {
        match self {
            TransactionSigner::AccountOwner(sk) => app_query
                .get_account_info(&sk.to_pk().into(), |a| a.nonce)
                .unwrap_or_default(),
            TransactionSigner::NodeMain(sk) => {
                let node_index = app_query.pubkey_to_index(&sk.to_pk()).unwrap();
                app_query
                    .get_node_info(&node_index, |n| n.nonce)
                    .unwrap_or_default()
            },
        }
    }
}
