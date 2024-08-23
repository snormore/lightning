use anyhow::Result;

use crate::StateTree;

pub trait StateTreeBuilder<T: StateTree> {
    fn build(self) -> Result<T::Writer>;
}
