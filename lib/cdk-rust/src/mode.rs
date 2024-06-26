pub struct PrimaryMode {
    pub(crate) _client_secret_key: [u8; 32],
    pub(crate) service_id: u32,
}

pub struct SecondaryMode {
    pub(crate) access_token: [u8; 48],
    pub(crate) _node_pk: [u8; 32],
}

pub enum ModeSetting {
    Primary(PrimaryMode),
    Secondary(SecondaryMode),
}

pub trait Mode: sealed::Sealed {}

impl Mode for PrimaryMode {}
impl Mode for SecondaryMode {}

mod sealed {
    use crate::mode::{PrimaryMode, SecondaryMode};

    pub trait Sealed {}

    impl Sealed for PrimaryMode {}
    impl Sealed for SecondaryMode {}
}
