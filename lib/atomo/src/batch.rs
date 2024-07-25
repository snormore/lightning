use std::ops::{Deref, DerefMut};

use fxhash::FxHashMap;

use crate::TableIndex;

pub type BoxedVec = Box<[u8]>;

pub type BatchHashMap = FxHashMap<BoxedVec, Operation>;

#[derive(Debug, Clone)]
/// A vertical batch contains a list of slots for each different table. Putting
/// the [`VerticalBatch`] into a [`SnapshotList`] will provide a valid snapshot
/// list.
pub struct VerticalBatch(Vec<BatchHashMap>);

#[derive(Debug, Clone)]
/// The change on a value.
pub enum Operation {
    Remove,
    Insert(BoxedVec),
}

impl VerticalBatch {
    /// Returns a new empty vertical batch with the given size. The
    /// size can be used for the number of tables.
    #[inline(always)]
    pub fn new(size: u8) -> Self {
        let size: usize = size.into();
        let mut vec = Vec::with_capacity(size);
        vec.resize_with(size, FxHashMap::default);
        VerticalBatch(vec)
    }

    /// Consume the vertical batch and returns the underlying vector of batches.
    #[inline(always)]
    pub fn into_raw(self) -> Vec<BatchHashMap> {
        self.0
    }

    #[inline(always)]
    pub fn get(&self, index: TableIndex) -> &BatchHashMap {
        let index: usize = index.into();
        debug_assert!(index < self.0.len());
        &self.0[index]
    }

    #[inline(always)]
    pub fn get_mut(&mut self, index: TableIndex) -> &mut BatchHashMap {
        let index: usize = index.into();
        debug_assert!(index < self.0.len());
        &mut self.0[index]
    }

    #[inline(always)]
    pub fn insert(&mut self, index: TableIndex, key: BoxedVec, operation: Operation) {
        let index: usize = index.into();
        self.0[index].insert(key, operation);
    }

    #[inline(always)]
    pub fn set(&mut self, index: TableIndex, batch: BatchHashMap) {
        let index: usize = index.into();
        self.0[index] = batch;
    }

    /// Return a reference to a single slot in the vertical batch.
    ///
    /// # Safety
    ///
    /// It is up to the caller to ensure:
    ///
    /// 1. The index is only claimed once.
    /// 2. The reference's lifetime is bounded to this [`VerticalBatch`].
    #[inline(always)]
    pub(crate) unsafe fn claim(&self, index: usize) -> BatchReference {
        let x = self.0.get_unchecked(index) as *const BatchHashMap as *mut BatchHashMap;
        BatchReference(x)
    }
}

/// The reference to a single batch slot.
pub(crate) struct BatchReference(*mut BatchHashMap);

impl BatchReference {
    #[inline(always)]
    pub fn as_mut(&mut self) -> &mut BatchHashMap {
        unsafe { &mut *self.0 }
    }
}

impl Deref for BatchReference {
    type Target = BatchHashMap;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl DerefMut for BatchReference {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set() {
        let mut batch = VerticalBatch::new(2);

        batch.insert(0, [1].into(), Operation::Insert([1].into()));
        batch.insert(1, [1].into(), Operation::Remove);

        let mut new_batch = BatchHashMap::default();
        new_batch.insert([3].into(), Operation::Insert([3].into()));
        new_batch.insert([4].into(), Operation::Insert([4].into()));

        batch.set(0, new_batch);
        assert_eq!(batch.0.len(), 2);
        assert_eq!(batch.0[0].len(), 2);
        assert_eq!(batch.0[1].len(), 1);
    }

    #[test]
    fn insert() {
        let mut batch = VerticalBatch::new(2);

        batch.insert(0, [1].into(), Operation::Insert([1].into()));
        assert_eq!(batch.0.len(), 2);
        assert_eq!(batch.0[0].len(), 1);
        assert_eq!(batch.0[1].len(), 0);

        batch.insert(1, [2].into(), Operation::Remove);
        assert_eq!(batch.0.len(), 2);
        assert_eq!(batch.0[0].len(), 1);
        assert_eq!(batch.0[1].len(), 1);

        batch.insert(0, [3].into(), Operation::Remove);
        assert_eq!(batch.0.len(), 2);
        assert_eq!(batch.0[0].len(), 2);
        assert_eq!(batch.0[1].len(), 1);
    }
}
