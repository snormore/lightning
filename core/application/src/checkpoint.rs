use derive_more::{From, IsVariant, TryInto};
use fleek_crypto::{ConsensusAggregateSignature, ConsensusSignature};
use lightning_interfaces::schema::LightningMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, IsVariant, From, TryInto)]
pub(crate) enum CheckpointMessage {
    // TODO(snormore): Is this enum necessary?
    CheckpointAttestation(CheckpointHeader),
}

impl LightningMessage for CheckpointMessage {
    fn encode<W: std::io::Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        // TODO(snormore): Implement this
        todo!("TODO(snormore):")
    }

    fn decode(_buffer: &[u8]) -> anyhow::Result<Self> {
        // TODO(snormore): Implement this
        todo!("TODO(snormore):")
    }

    fn encode_length_delimited<W: std::io::Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        // TODO(snormore): Implement this
        todo!("TODO(snormore):")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointHeader {
    // TODO(snormore): Hash types.
    pub previous_state: [u8; 32],
    pub next_state: [u8; 32],
    pub signature: ConsensusSignature, // TODO(should): Should this just be [u8; 48]?
}

impl CheckpointHeader {
    pub fn new(
        previous_state: [u8; 32],
        next_state: [u8; 32],
        signature: ConsensusSignature,
    ) -> Self {
        Self {
            previous_state,
            next_state,
            signature,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggrCheckpointHeader {
    pub prev_state: [u8; 32],
    pub next_state: [u8; 32],
    pub signature: ConsensusAggregateSignature, // TODO(should): Should this just be [u8; 48]?
    pub nodes: Vec<u8>,                         // TODO(snormore): BitSet
}
