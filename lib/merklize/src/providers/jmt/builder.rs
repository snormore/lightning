use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use atomo::AtomoBuilder;
use jmt::storage::{Node, NodeKey};
use jmt::KeyHash;

use super::tree::{KEYS_TABLE_NAME, NODES_TABLE_NAME};
use crate::writer::StateTreeWriter;
use crate::{StateKey, StateTree, StateTreeBuilder};

pub struct JmtStateTreeBuilder<T: StateTree> {
    inner: AtomoBuilder<T::StorageBuilder, T::Serde>,

    _tree: PhantomData<T>,
}

impl<T: StateTree> JmtStateTreeBuilder<T> {
    pub fn new(inner: AtomoBuilder<T::StorageBuilder, T::Serde>) -> Self {
        Self {
            inner,

            _tree: PhantomData,
        }
    }
}

impl<T: StateTree> StateTreeBuilder<T> for JmtStateTreeBuilder<T> {
    fn build(self) -> Result<T::Writer> {
        let db = self
            .inner
            .with_table::<NodeKey, Node>(NODES_TABLE_NAME)
            .with_table::<KeyHash, StateKey>(KEYS_TABLE_NAME)
            .build()
            .map_err(|e| anyhow!("{:?}", e))?;

        // TODO(snormore): Should the writer just have `build` and we can delete this builder
        // completely?
        Ok(T::Writer::new(db))
    }
}
