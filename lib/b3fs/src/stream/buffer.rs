use std::borrow::Borrow;
use std::fmt::Debug;

use thiserror::Error;

use super::encoder::ProofEncoder;
use super::iter::ProofBufIter;
use super::pretty::ProofBufPrettyPrinter;
use super::walker::{Mode, TreeWalker};
use crate::collections::HashTree;
use crate::utils::is_valid_proof_len;

/// A buffer containing a proof for a block of data. This allows us to have a pre-allocated vec
/// and insert from right to left and still deref to the correct slice on the right bounds.
pub struct ProofBuf {
    /// The index at which the slice starts at in the boxed buffer.
    pub(crate) index: usize,
    /// The allocated storage of this buffer.
    pub(crate) buffer: Box<[u8]>,
}

impl ProofBuf {
    fn new_internal(tree: HashTree, walker: TreeWalker) -> Self {
        let size = walker.size_hint().0;
        let array = tree.as_inner();
        let mut encoder = ProofEncoder::new(size);
        for (direction, index) in walker {
            debug_assert!(index < tree.inner_len(), "Index overflow.");
            encoder.insert(direction, &array[index]);
        }
        encoder.finalize()
    }

    /// Create a proof with the provided configurations.
    pub fn new(mode: Mode, tree: HashTree, block: usize) -> Self {
        match mode {
            Mode::Initial => Self::initial(tree, block),
            Mode::Proceeding => Self::proceeding(tree, block),
        }
    }

    /// Construct a new proof for the given block index from the provided tree.
    pub fn initial(tree: HashTree, block: usize) -> Self {
        Self::new_internal(tree, TreeWalker::initial(block, tree.inner_len()))
    }

    /// Construct proof for the given block number assuming that previous
    /// blocks have already been sent.
    pub fn proceeding(tree: HashTree, block: usize) -> Self {
        Self::new_internal(tree, TreeWalker::proceeding(block, tree.inner_len()))
    }

    /// Returns the proof as a slice.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.index..]
    }

    /// Returns the length of the proof.
    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len() - self.index
    }

    /// Returns if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over this proof buffer.
    #[inline]
    pub fn iter(&self) -> ProofBufIter<'_> {
        ProofBufIter::new(self.as_slice())
    }
}

impl AsRef<[u8]> for ProofBuf {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for ProofBuf {
    #[inline(always)]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

#[derive(Copy, Clone, Debug, Error)]
#[error("Invalid proof length.")]
pub struct InvalidProofSize;

impl TryFrom<Box<[u8]>> for ProofBuf {
    type Error = InvalidProofSize;

    fn try_from(value: Box<[u8]>) -> Result<Self, Self::Error> {
        if !is_valid_proof_len(value.len()) {
            Err(InvalidProofSize)
        } else {
            Ok(Self {
                index: 0,
                buffer: value,
            })
        }
    }
}

impl TryFrom<Vec<u8>> for ProofBuf {
    type Error = InvalidProofSize;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(value.into_boxed_slice())
    }
}

impl Debug for ProofBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        ProofBufPrettyPrinter(self.as_slice()).fmt(f)
    }
}

impl PartialEq<&[u8]> for ProofBuf {
    fn eq(&self, other: &&[u8]) -> bool {
        self.as_slice().eq(*other)
    }
}

impl PartialEq<ProofBuf> for ProofBuf {
    fn eq(&self, other: &ProofBuf) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}
