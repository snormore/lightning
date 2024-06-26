use std::net::SocketAddrV4;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::bail;
use aya::maps::{HashMap, MapData};
use lightning_ebpf_common::{
    File,
    FileRule,
    PacketFilter,
    PacketFilterParams,
    Profile,
    MAX_BUFFER_LEN,
    MAX_FILE_RULES,
};
use log::debug;
use tokio::fs;
use tokio::sync::Mutex;

use crate::config::{ConfigSource, GLOBAL_PROFILE};
use crate::map::PacketFilterRule;

#[derive(Clone)]
pub struct SharedMap {
    packet_filters: Arc<Mutex<HashMap<MapData, PacketFilter, PacketFilterParams>>>,
    file_open_rules: Arc<Mutex<HashMap<MapData, File, Profile>>>,
    config_src: ConfigSource,
}

impl SharedMap {
    pub fn new(
        packet_filters: HashMap<MapData, PacketFilter, PacketFilterParams>,
        file_open_rules: HashMap<MapData, File, Profile>,
        config_src: ConfigSource,
    ) -> Self {
        Self {
            packet_filters: Arc::new(Mutex::new(packet_filters)),
            file_open_rules: Arc::new(Mutex::new(file_open_rules)),
            config_src,
        }
    }

    pub async fn packet_filter_add(&mut self, addr: SocketAddrV4) -> anyhow::Result<()> {
        let mut map = self.packet_filters.lock().await;
        map.insert(
            PacketFilter {
                ip: u32::from_be_bytes(addr.ip().octets()),
                port: addr.port(),
                proto: PacketFilterRule::TCP,
            },
            PacketFilterParams {
                trigger_event: 1,
                shortlived: 1,
                action: PacketFilterRule::DROP,
            },
            0,
        )?;
        Ok(())
    }

    pub async fn packet_filter_remove(&mut self, addr: SocketAddrV4) -> anyhow::Result<()> {
        let mut map = self.packet_filters.lock().await;
        map.remove(&PacketFilter {
            ip: u32::from_be_bytes(addr.ip().octets()),
            port: addr.port(),
            proto: PacketFilterRule::TCP,
        })?;
        Ok(())
    }

    /// Updates packet filters.
    ///
    /// Reads from disk so it's a heavy operation.
    pub async fn update_packet_filters(&self) -> anyhow::Result<()> {
        let filters: Vec<PacketFilterRule> = self.config_src.read_packet_filters().await?;
        let new_state = filters
            .into_iter()
            .map(|filter| (PacketFilter::from(filter), PacketFilterParams::from(filter)))
            .collect::<std::collections::HashMap<_, _>>();

        let mut map = self.packet_filters.lock().await;
        // Due to a constraint of the aya api, there is no clean method for the maps and
        // we don't get mutable access as iterator is read only.
        let mut remove = Vec::new();
        for result in map.iter() {
            let (filter, params) = result?;
            // Filters with shortlived=1 do not get removed.
            // This is to support dynamic ephemiral rules
            // that may be produced by rate limiting, for example.
            if !new_state.contains_key(&filter) && params.shortlived != 1 {
                remove.push(filter);
            }
        }

        for (filter, params) in new_state {
            map.insert(filter, params, 0)?;
        }

        for filter in remove {
            map.remove(&filter)?;
        }

        Ok(())
    }

    /// Updates file rules.
    ///
    /// Reads from disk so it's a heavy operation.
    pub async fn update_all_file_rules(&self) -> anyhow::Result<()> {
        let profiles = self.config_src.get_profiles().await?;

        let mut new = std::collections::HashMap::new();
        for profile in profiles {
            let exec_path = profile.name.as_ref().unwrap_or(&GLOBAL_PROFILE);
            let (exec, _) = file_from_path(exec_path).await?;
            let mut rules = vec![lightning_ebpf_common::FileRule::default(); MAX_FILE_RULES];
            for (i, rule) in profile.file_rules.iter().enumerate() {
                let (file, is_dir) = file_from_path(&rule.file).await?;
                if exec.dev != file.dev {
                    // Protecting files in more than one device is not supported yet.
                    bail!("executable file device and file device do not match");
                }

                if i >= MAX_FILE_RULES {
                    bail!("path maximum {MAX_FILE_RULES} execeeded");
                }

                let mut vector = vec![0u8; MAX_BUFFER_LEN];
                let path = rule.file.as_path().display().to_string();

                debug!("path {path} for profile {}", exec_path.display());

                vector[..path.len()].copy_from_slice(path.as_bytes());

                rules[i].path = vector.try_into().expect("Size is hardcoded");
                rules[i].is_dir = if is_dir {
                    FileRule::IS_DIR
                } else {
                    FileRule::IS_FILE
                };
                rules[i].permissions = rule.permissions;
            }

            let rules: [lightning_ebpf_common::FileRule; MAX_FILE_RULES] =
                rules.try_into().expect("Vec len is hardcoded");
            new.insert(exec, Profile { rules });
        }

        let mut maps = self.file_open_rules.lock().await;

        // Due to a constraint of the aya api, there is no clean method for the maps
        // so we remove all of them. Todo: Let's open an issue with aya.
        let mut remove = Vec::new();
        for file in maps.keys() {
            remove.push(file);
        }
        for file in remove {
            let f = file?;
            maps.remove(&f)?;
        }

        for (exec, rules) in new {
            maps.insert(exec, rules, 0)?;
        }

        Ok(())
    }

    pub async fn update_file_rules(&self, path: PathBuf) -> anyhow::Result<()> {
        let profile = self.config_src.read_profile(Some(path.as_os_str())).await?;
        let exec_path = profile.name.as_ref().unwrap_or(&GLOBAL_PROFILE);
        let (exec, _) = file_from_path(exec_path).await?;
        let mut file_open_rules = vec![lightning_ebpf_common::FileRule::default(); MAX_FILE_RULES];
        for (i, rule) in profile.file_rules.iter().enumerate() {
            let (file, is_dir) = file_from_path(&rule.file).await?;
            if exec.dev != file.dev {
                // Protecting files in more than one device is not supported yet.
                bail!("executable file device and file device do not match");
            }
            if i >= MAX_FILE_RULES {
                bail!("path maximum {MAX_FILE_RULES} exceeded");
            }

            let mut vector = vec![0u8; MAX_BUFFER_LEN];
            let path = rule.file.as_path().display().to_string();
            vector[..path.len()].copy_from_slice(path.as_bytes());

            debug!("path {path} for profile {}", exec_path.display());

            file_open_rules[i].path = vector.try_into().expect("Size is hardcoded");
            file_open_rules[i].is_dir = if is_dir {
                FileRule::IS_DIR
            } else {
                FileRule::IS_FILE
            };
            file_open_rules[i].permissions = rule.permissions;
        }

        let rules: [lightning_ebpf_common::FileRule; MAX_FILE_RULES] =
            file_open_rules.try_into().expect("Vec len is hardcoded");

        let mut maps = self.file_open_rules.lock().await;
        maps.insert(&exec, Profile { rules }, 0)?;

        Ok(())
    }
}

async fn file_from_path(path: &PathBuf) -> anyhow::Result<(File, bool)> {
    let file = fs::File::open(path.as_path()).await?;
    let metadata = file.metadata().await?;
    let is_dir = metadata.is_dir();
    let inode = metadata.ino();
    Ok((File::new(inode), is_dir))
}
