use std::marker::PhantomData;

use async_trait::async_trait;
use lightning_interfaces::infu_collection::Collection;
use lightning_interfaces::{
    ConfigConsumer,
    ExecutorProviderInterface,
    ServiceExecutorInterface,
    WithStartAndShutdown,
};
use serde::{Deserialize, Serialize};

use crate::deque::CommandStealer;
use crate::handle::ServiceHandle;

pub struct ServiceExecutor<C: Collection> {
    collection: PhantomData<C>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct ServiceExecutorConfig {}

#[derive(Clone)]
pub struct Provider {}

impl<C: Collection> ServiceExecutorInterface<C> for ServiceExecutor<C> {
    type Provider = Provider;

    fn init(_config: Self::Config) -> anyhow::Result<Self> {
        todo!()
    }

    fn get_provider(&self) -> Self::Provider {
        todo!()
    }
}

#[async_trait]
impl<C: Collection> WithStartAndShutdown for ServiceExecutor<C> {
    /// Returns true if this system is running or not.
    fn is_running(&self) -> bool {
        true
    }

    /// Start the system, should not do anything if the system is already
    /// started.
    async fn start(&self) {}

    /// Send the shutdown signal to the system.
    async fn shutdown(&self) {}
}

impl<C: Collection> ConfigConsumer for ServiceExecutor<C> {
    const KEY: &'static str = "service-executor";
    type Config = ServiceExecutorConfig;
}

impl ExecutorProviderInterface for Provider {
    type Handle = ServiceHandle;
    type Stealer = CommandStealer;

    fn get_work_stealer(&self) -> Self::Stealer {
        todo!()
    }

    fn get_service_handle(
        &self,
        _service_id: lightning_interfaces::types::ServiceId,
    ) -> Option<Self::Handle> {
        todo!()
    }
}
