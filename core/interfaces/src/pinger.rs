use fdi::BuildGraph;

#[interfaces_proc::blank]
pub trait PingerInterface: BuildGraph + Sized + Send + Sync {}
