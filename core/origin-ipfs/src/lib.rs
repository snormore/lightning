// TODO(snormore): Remove this and fix the issue.
#![feature(never_type, never_type_fallback)]

mod car_reader;
pub mod config;
mod decoder;
mod error;
mod origin_ipfs;
#[cfg(test)]
mod tests;

pub use config::Config;
pub use origin_ipfs::IPFSOrigin;
