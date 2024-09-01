use fdi::BuildGraph;
use ready::empty::EmptyReadyState;
use ready::ReadyWaiterState;

use crate::collection::Collection;
use crate::ConfigConsumer;

#[interfaces_proc::blank]
pub trait CheckpointerInterface<C: Collection>: BuildGraph + ConfigConsumer + Send + Sync {
    #[blank(EmptyReadyState)]
    type ReadyState: ReadyWaiterState;

    async fn wait_for_ready(&self) -> Self::ReadyState;
}
