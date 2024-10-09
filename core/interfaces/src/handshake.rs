use fdi::BuildGraph;

#[interfaces_proc::blank]
pub trait HandshakeInterface: BuildGraph + Sized + Send + Sync {}
