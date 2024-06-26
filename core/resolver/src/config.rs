use lightning_utils::config::LIGHTNING_HOME_DIR;
use resolved_pathbuf::ResolvedPathBuf;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    /// Path to the database used by the resolver.
    pub store_path: ResolvedPathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_path: LIGHTNING_HOME_DIR
                .join("data/resolver_store")
                .try_into()
                .expect("Failed to resolve path"),
        }
    }
}
