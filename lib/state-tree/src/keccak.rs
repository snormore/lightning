use jmt::SimpleHasher;
use tiny_keccak::{Hasher, Keccak};

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
