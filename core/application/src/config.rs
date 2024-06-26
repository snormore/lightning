use std::time::SystemTime;

use anyhow::{anyhow, Result};
use lightning_utils::config::LIGHTNING_HOME_DIR;
use resolved_pathbuf::ResolvedPathBuf;
use serde::{Deserialize, Serialize};

use crate::genesis::Genesis;
use crate::network::Network;

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: Option<Network>,
    pub genesis_path: Option<ResolvedPathBuf>,
    pub storage: StorageConfig,
    pub db_path: Option<ResolvedPathBuf>,
    pub db_options: Option<ResolvedPathBuf>,

    // Development options.
    // Should not be used in production, and will likely break your node if you do.
    pub dev: Option<DevConfig>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DevConfig {
    // Whether to update the genesis epoch start to the current time when starting the node.
    pub update_epoch_start_to_now: bool,
}

impl Default for DevConfig {
    fn default() -> Self {
        Self {
            update_epoch_start_to_now: true,
        }
    }
}

impl Config {
    pub fn test(genesis_path: ResolvedPathBuf) -> Self {
        Self {
            network: None,
            genesis_path: Some(genesis_path),
            storage: StorageConfig::InMemory,
            db_path: None,
            db_options: None,
            dev: None,
        }
    }

    pub fn genesis(&self) -> Result<Genesis> {
        let mut genesis = match &self.network {
            Some(network) => match &self.genesis_path {
                Some(_genesis_path) => Err(anyhow!(
                    "Cannot specify both network and genesis_path in config"
                )),
                None => network.genesis(),
            },
            None => match &self.genesis_path {
                Some(genesis_path) => Ok(Genesis::load_from_file(genesis_path.clone())?),
                None => Err(anyhow!("Missing network in config")),
            },
        }?;
        if let Some(dev) = &self.dev {
            if dev.update_epoch_start_to_now {
                genesis.epoch_start = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64
            }
        }
        Ok(genesis)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: None,
            genesis_path: None,
            storage: StorageConfig::RocksDb,
            db_path: Some(
                LIGHTNING_HOME_DIR
                    .join("data/app_db")
                    .try_into()
                    .expect("Failed to resolve path"),
            ),
            db_options: None,
            dev: None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum StorageConfig {
    InMemory,
    RocksDb,
}

#[cfg(test)]
mod config_tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn genesis_with_network_without_genesis() {
        let config = Config {
            network: Some(Network::LocalnetExample),
            genesis_path: None,
            ..Config::default()
        };
        assert!(config.genesis().is_ok());
    }

    #[test]
    fn genesis_without_network_with_genesis() {
        let temp_dir = tempdir().unwrap();
        let genesis_path = Genesis::default()
            .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
            .unwrap();
        let config = Config {
            network: None,
            genesis_path: Some(genesis_path),
            ..Config::default()
        };
        assert!(config.genesis().is_ok());
    }

    #[test]
    fn genesis_missing_network_and_genesis() {
        let config = Config {
            network: None,
            genesis_path: None,
            ..Config::default()
        };
        assert!(config.genesis().is_err());
    }

    #[test]
    fn genesis_with_network_and_genesis() {
        let temp_dir = tempdir().unwrap();
        let genesis_path = Genesis::default()
            .write_to_dir(temp_dir.path().to_path_buf().try_into().unwrap())
            .unwrap();
        let config = Config {
            network: Some(Network::LocalnetExample),
            genesis_path: Some(genesis_path),
            ..Config::default()
        };
        assert!(config.genesis().is_err());
    }
}
