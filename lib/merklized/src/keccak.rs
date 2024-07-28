use tiny_keccak::{Hasher, Keccak};

use crate::SimpleHasher;

#[derive(Clone)]
pub struct KeccakHasher(Keccak);

impl SimpleHasher for KeccakHasher {
    fn new() -> Self {
        KeccakHasher(Keccak::v256())
    }

    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self) -> [u8; 32] {
        let mut output = [0u8; 32];
        self.0.finalize(&mut output);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keccak_hasher() {
        let mut hasher = KeccakHasher::new();
        hasher.update(b"hello");
        hasher.update(b"world");
        let hash = hasher.finalize();
        assert_eq!(
            hash,
            [
                250, 38, 219, 124, 168, 94, 173, 57, 146, 22, 231, 198, 49, 107, 197, 14, 210, 67,
                147, 195, 18, 43, 88, 39, 53, 231, 243, 176, 249, 27, 147, 240
            ]
        );
        assert_eq!(hash.len(), 32);
    }
}
