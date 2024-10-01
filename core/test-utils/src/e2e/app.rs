use fleek_crypto::{
    AccountOwnerSecretKey,
    NodeSecretKey,
    SecretKey,
    TransactionSender,
    TransactionSignature,
};
use lightning_interfaces::types::{ChainId, UpdateMethod, UpdatePayload, UpdateRequest};
use lightning_interfaces::ToDigest;

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
}

/// Build and sign a new update transaction.
pub fn new_update_transaction(
    method: UpdateMethod,
    chain_id: ChainId,
    nonce: u64,
    signer: TransactionSigner,
) -> UpdateRequest {
    let payload = UpdatePayload {
        sender: signer.to_sender(),
        nonce,
        method,
        chain_id,
    };
    let digest = payload.to_digest();
    let signature = signer.sign(&digest);

    UpdateRequest { payload, signature }
}
