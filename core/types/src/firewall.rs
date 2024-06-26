use std::net::IpAddr;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FirewallConfig {
    pub name: String,
    pub connection_policy: ConnectionPolicyConfig,
    pub rate_limiting: RateLimitingConfig,
}

impl FirewallConfig {
    pub fn none(name: String) -> Self {
        Self {
            name,
            connection_policy: ConnectionPolicyConfig::All,
            rate_limiting: RateLimitingConfig::None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ConnectionPolicyConfig {
    All,
    Whitelist { members: Vec<IpAddr> },
    Blacklist { members: Vec<IpAddr> },
}

/// The connection policy for the firewall
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RateLimitingConfig {
    None,
    Per,
    Global { rules: Vec<RateLimitingRule> },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RateLimitingRule {
    pub period: Period,
    pub max_requests: u64,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub enum Period {
    Second,
    Minute,
    Hour,
    Day,
}

impl Period {
    pub const MAX: Self = Self::Day;

    pub fn as_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.as_secs())
    }

    pub const fn as_secs(&self) -> u64 {
        match self {
            Period::Second => 1,
            Period::Minute => 60,
            Period::Hour => 3600,
            Period::Day => 86400,
        }
    }

    pub const fn as_millis(&self) -> u64 {
        self.as_secs() * 1000
    }
}
