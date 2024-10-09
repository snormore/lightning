use fdi::BuildGraph;
use lightning_types::Blake3Hash;

#[interfaces_proc::blank]
pub trait IndexerInterface: BuildGraph + Clone + Send + Sync + Sized {
    async fn register(&self, cid: Blake3Hash);

    async fn unregister(&self, cid: Blake3Hash);
}
