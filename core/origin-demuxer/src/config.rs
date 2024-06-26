use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct Config {
    pub http: lightning_origin_http::Config,
    pub ipfs: lightning_origin_ipfs::Config,
}
