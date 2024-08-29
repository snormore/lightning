use derive_more::{From, IsVariant, TryInto};
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
        todo!()
    }

    fn decode(_buffer: &[u8]) -> anyhow::Result<Self> {
        // TODO(snormore): Implement this
        todo!()
    }

    fn encode_length_delimited<W: std::io::Write>(&self, _writer: &mut W) -> std::io::Result<()> {
        // TODO(snormore): Implement this
        todo!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointHeader {
    // TODO(snormore): Hash types.
    previous_state: [u8; 32],
    next_state: [u8; 32],
    signature: Vec<u8>, // bls signature
}

impl CheckpointHeader {
    pub fn new(previous_state: [u8; 32], next_state: [u8; 32], signature: Vec<u8>) -> Self {
        Self {
            previous_state,
            next_state,
            signature,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggrCheckpointHeader {
    prev_state: [u8; 32],
    next_state: [u8; 32],
    signature: Vec<u8>, // bls aggr signature
    nodes: Vec<u8>,     // TODO: BitSet
}
