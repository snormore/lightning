use std::path::Path;
use std::time::SystemTime;

use anyhow::{anyhow, Context, Result};
use atomo::{AtomoBuilder, DefaultSerdeBackend};
use atomo_rocks::{Cache as RocksCache, Env as RocksEnv, Options};
use lightning_application_state::storage::AtomoStorageBuilder;
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

    pub fn atomo_builder<'a>(
        &'a self,
        checkpoint: Option<([u8; 32], &'a [u8])>,
    ) -> Result<AtomoBuilder<AtomoStorageBuilder, DefaultSerdeBackend>> {
        let storage = match self.storage {
            StorageConfig::RocksDb => {
                let db_path = self
                    .db_path
                    .as_ref()
                    .context("db_path must be specified for RocksDb backend")?;
                let mut db_options = if let Some(db_options) = self.db_options.as_ref() {
                    let (options, _) = Options::load_latest(
                        db_options,
                        RocksEnv::new().context("Failed to create rocks db env.")?,
                        false,
                        // TODO(matthias): I set this lru cache size arbitrarily
                        RocksCache::new_lru_cache(100),
                    )
                    .context("Failed to create rocks db options.")?;
                    options
                } else {
                    Options::default()
                };
                db_options.create_if_missing(true);
                db_options.create_missing_column_families(true);
                match checkpoint {
                    Some((hash, checkpoint)) => AtomoStorageBuilder::new(Some(db_path.as_path()))
                        .with_options(db_options)
                        .from_checkpoint(hash, checkpoint),
                    None => {
                        AtomoStorageBuilder::new(Some(db_path.as_path())).with_options(db_options)
                    },
                }
            },
            StorageConfig::InMemory => AtomoStorageBuilder::new::<&Path>(None),
        };

        let atomo = AtomoBuilder::<AtomoStorageBuilder, DefaultSerdeBackend>::new(storage);

        Ok(atomo)
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
