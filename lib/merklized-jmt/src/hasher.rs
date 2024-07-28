use merklized::SimpleHasher;

pub(crate) struct SimpleHasherWrapper<H: SimpleHasher>(H);

impl<H: SimpleHasher> jmt::SimpleHasher for SimpleHasherWrapper<H> {
    fn new() -> Self {
        SimpleHasherWrapper(H::new())
    }

    fn update(&mut self, data: &[u8]) {
        self.0.update(data)
    }

    fn finalize(self) -> [u8; 32] {
        self.0.finalize()
    }
}
