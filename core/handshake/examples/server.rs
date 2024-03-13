// cargo run --example server -- --config ./examples/config.toml -vvvv run
// run static file server separately, e.g
// python3 -m http.server

use lightning_blockstore::blockstore::Blockstore;
use lightning_cli::cli::Cli;
use lightning_handshake::handshake::Handshake;
use lightning_interfaces::infu_collection::Collection;
use lightning_interfaces::partial;
use lightning_node::config::TomlConfigProvider;
use lightning_rpc::Rpc;
use lightning_service_executor::shim::ServiceExecutor;
use lightning_signer::Signer;
use mock::syncronizer::MockSyncronizer;

partial!(ExampleBinding {
    ConfigProviderInterface = TomlConfigProvider<Self>;
    BlockStoreInterface = Blockstore<Self>;
    SignerInterface = Signer<Self>;
    SyncronizerInterface = MockSyncronizer<Self>;
    HandshakeInterface = Handshake<Self>;
    ServiceExecutorInterface = ServiceExecutor<Self>;
    RpcInterface = Rpc<Self>;
});

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    cli.exec().await
}