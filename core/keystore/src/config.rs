use fleek_crypto::{ConsensusSecretKey, NodeSecretKey, SecretKey};
use lightning_utils::config::LIGHTNING_HOME_DIR;
use resolved_pathbuf::ResolvedPathBuf;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeystoreConfig {
    pub node_key_path: ResolvedPathBuf,
    pub consensus_key_path: ResolvedPathBuf,
}

impl Default for KeystoreConfig {
    fn default() -> Self {
        Self {
            node_key_path: LIGHTNING_HOME_DIR
                .join("keystore/node.pem")
                .try_into()
                .expect("Failed to resolve path."),
            consensus_key_path: LIGHTNING_HOME_DIR
                .join("keystore/consensus.pem")
                .try_into()
                .expect("Failed to resolve path."),
        }
    }
}

impl KeystoreConfig {
    pub fn test() -> Self {
        Self {
            node_key_path: "../test-utils/keys/test_node.pem"
                .try_into()
                .expect("Failed to resolve path."),
            consensus_key_path: "../test-utils/keys/test_consensus.pem"
                .try_into()
                .expect("Failed to resolve path."),
        }
    }

    pub fn test2() -> Self {
        Self {
            node_key_path: "../test-utils/keys/test_node2.pem"
                .try_into()
                .expect("Failed to resolve path."),
            consensus_key_path: "../test-utils/keys/test_consensus2.pem"
                .try_into()
                .expect("Failed to resolve path."),
        }
    }

    pub fn load_test_keys(&self) -> (ConsensusSecretKey, NodeSecretKey) {
        let encoded_node = std::fs::read_to_string(self.node_key_path.clone())
            .expect("Failed to read node pem file");

        let encoded_consensus = std::fs::read_to_string(self.consensus_key_path.clone())
            .expect("Failed to read consensus pem file");

        (
            ConsensusSecretKey::decode_pem(&encoded_consensus)
                .expect("Failed to decode consensus pem file"),
            NodeSecretKey::decode_pem(&encoded_node).expect("Failed to decode node pem file"),
        )
    }
}
