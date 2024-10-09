use affair::Socket;
use fdi::BuildGraph;
use lightning_types::{FetcherRequest, FetcherResponse};

pub type FetcherSocket = Socket<FetcherRequest, FetcherResponse>;

#[interfaces_proc::blank]
pub trait FetcherInterface: BuildGraph + Sized + Send + Sync {
    /// Returns a socket that can be used to submit requests to the fetcher.
    #[socket]
    fn get_socket(&self) -> FetcherSocket;
}
