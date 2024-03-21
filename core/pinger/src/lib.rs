pub mod config;
pub mod pinger;

pub use config::Config;
pub use pinger::Pinger;

// TODO(qti3e): We should test the pinger implementation. The test should actually check that
// the server works and a client does in fact get the data it needs from a server without any
// crashes.
