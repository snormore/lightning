use std::net::SocketAddr;

use ready::{ReadyWaiter, ReadyWaiterState};

pub type PoolReadyWaiter = ReadyWaiter<PoolReadyState>;

#[derive(Debug, Default, Clone)]
pub struct PoolReadyState {
    pub listen_address: Option<SocketAddr>,
}

impl ReadyWaiterState for PoolReadyState {}
