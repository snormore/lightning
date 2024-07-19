use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use atomo::{StorageBackend, StorageBackendConstructor, TableId};
use jmt::SimpleHasher;

use crate::StateTreeWriter;

pub struct StateTreeBuilder<C: StorageBackendConstructor, KH: SimpleHasher, VH: SimpleHasher> {
    constructor: C,
    table_id_by_name: HashMap<String, TableId>,
    _kv_hashers: PhantomData<(KH, VH)>,
}

impl<C: StorageBackendConstructor, KH: SimpleHasher, VH: SimpleHasher> StateTreeBuilder<C, KH, VH> {
    pub fn new(constructor: C) -> Self {
        Self {
            constructor,
            table_id_by_name: Default::default(),
            _kv_hashers: PhantomData,
        }
    }
}

impl<C: StorageBackendConstructor, KH: SimpleHasher, VH: SimpleHasher> StorageBackendConstructor
    for StateTreeBuilder<C, KH, VH>
where
    C::Storage: StorageBackend + Send + Sync,
{
    type Storage = StateTreeWriter<C::Storage, KH, VH>;

    type Error = C::Error;

    fn open_table(&mut self, name: String) -> TableId {
        if let Some(table_id) = self.table_id_by_name.get(&name) {
            return *table_id;
        }
        let table_id = self.constructor.open_table(name.clone());
        self.table_id_by_name.insert(name, table_id);
        table_id
    }

    fn build(mut self) -> Result<Self::Storage, Self::Error> {
        let nodes_table_index = self
            .constructor
            .open_table(String::from("%state_tree_nodes"));

        let storage = Arc::new(self.constructor.build()?);

        Ok(StateTreeWriter::<_, KH, VH>::new(
            storage,
            nodes_table_index,
            self.table_id_by_name,
        ))
    }
}

impl<C: StorageBackendConstructor, KH: SimpleHasher, VH: SimpleHasher> Default
    for StateTreeBuilder<C, KH, VH>
where
    C: Default,
{
    fn default() -> Self {
        Self::new(C::default())
    }
}
