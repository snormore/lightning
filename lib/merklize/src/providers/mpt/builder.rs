use std::marker::PhantomData;

use anyhow::{anyhow, Result};
use atomo::AtomoBuilder;
use trie_db::DBValue;

use super::hasher::SimpleHasherWrapper;
use super::tree::{NODES_TABLE_NAME, ROOT_TABLE_NAME};
use crate::writer::StateTreeWriter;
use crate::{StateRootHash, StateTree, StateTreeBuilder};

pub struct MptStateTreeBuilder<T: StateTree> {
    inner: AtomoBuilder<T::StorageBuilder, T::Serde>,

    _tree: PhantomData<T>,
}

impl<T: StateTree> MptStateTreeBuilder<T> {
    pub fn new(inner: AtomoBuilder<T::StorageBuilder, T::Serde>) -> Self {
        Self {
            inner,

            _tree: PhantomData,
        }
    }
}

impl<T: StateTree> StateTreeBuilder<T> for MptStateTreeBuilder<T> {
    fn build(self) -> Result<T::Writer> {
        let db = self
            .inner
            .with_table::<<SimpleHasherWrapper<T::Hasher> as hash_db::Hasher>::Out, DBValue>(
                NODES_TABLE_NAME,
            )
            .with_table::<u8, StateRootHash>(ROOT_TABLE_NAME)
            .build()
            .map_err(|e| anyhow!("{:?}", e))?;

        Ok(T::Writer::new(db))
    }
}
