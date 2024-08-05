use atomo::{DefaultSerdeBackend, SerdeBackend, UpdatePerm};
use fleek_crypto::{EthAddress, NodePublicKey};
use lightning_application::env::Env;
use lightning_application::storage::AtomoStorage;
use lightning_types::{AccountInfo, NodeIndex, NodeInfo};
use merklize::hashers::blake3::Blake3Hasher;
use merklize::hashers::keccak::KeccakHasher;
use merklize::hashers::sha2::Sha256Hasher;
use merklize::providers::jmt::JmtMerklizeProvider;
use merklize::providers::mpt::MptMerklizeProvider;
use merklize::{MerklizeProvider, MerklizedAtomo, StateProof};
use merklize_test_utils::application::{create_rocksdb_env, new_complex_block, DummyPutter};
use tempfile::tempdir;

// JMT

#[tokio::test]
async fn test_application_jmt_rocksdb_blake3() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        JmtMerklizeProvider<AtomoStorage, DefaultSerdeBackend, Blake3Hasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

#[tokio::test]
async fn test_application_jmt_rocksdb_keccak256() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        JmtMerklizeProvider<AtomoStorage, DefaultSerdeBackend, KeccakHasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

#[tokio::test]
async fn test_application_jmt_rocksdb_sha256() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        JmtMerklizeProvider<AtomoStorage, DefaultSerdeBackend, Sha256Hasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

// MPT

#[tokio::test]
async fn test_application_mpt_rocksdb_blake3() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        MptMerklizeProvider<AtomoStorage, DefaultSerdeBackend, Blake3Hasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

#[tokio::test]
async fn test_application_mpt_rocksdb_keccak256() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        MptMerklizeProvider<AtomoStorage, DefaultSerdeBackend, KeccakHasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

#[tokio::test]
async fn test_application_mpt_rocksdb_sha256() {
    let temp_dir = tempdir().unwrap();
    let mut env = create_rocksdb_env::<
        MptMerklizeProvider<AtomoStorage, DefaultSerdeBackend, Sha256Hasher>,
    >(&temp_dir);

    test_application(&mut env).await;
}

async fn test_application<M>(env: &mut Env<UpdatePerm, AtomoStorage, M>)
where
    M: MerklizeProvider<Storage = AtomoStorage, Serde = DefaultSerdeBackend>,
{
    let (block, _stake_amount, eth_addresses, node_public_keys) = new_complex_block();

    env.run(block.clone(), || DummyPutter {}).await;

    let query = MerklizedAtomo::<_, _, _, M>::new(env.inner.query());

    let state_root = query.run(|ctx| M::get_state_root(ctx).unwrap());

    // Check that all accounts are present in the state tree.
    for eth_address in eth_addresses.iter() {
        query.run(|ctx| {
            let accounts_table = ctx.get_table::<EthAddress, AccountInfo>("account");
            let value = accounts_table.get(eth_address).unwrap();

            // Generate proof of existence.
            let proof =
                M::get_state_proof(ctx, "account", M::Serde::serialize(&eth_address)).unwrap();

            // Verify proof of existence.
            proof
                .verify_membership::<EthAddress, AccountInfo, M>(
                    "account",
                    eth_address,
                    value,
                    state_root,
                )
                .unwrap();
        });
    }

    // Check proof of non-existence for an account that does not exist.
    // TODO(snormore): Figure out why this test fails sometimes when using ics23 proofs in JMT, but
    // not when using `jmt::SparseMerkleProof`.
    // let non_existent_eth_address: EthAddress = {
    //     let secret_key = AccountOwnerSecretKey::generate();
    //     let public_key = secret_key.to_pk();
    //     public_key.into()
    // };
    // query.run(|ctx| {
    //     let accounts_table = ctx.get_table::<EthAddress, AccountInfo>("account");

    //     // Verify that the account does not exist.
    //     assert!(accounts_table.get(non_existent_eth_address).is_none());

    //     // Generate proof of non-existence.
    //     let proof = M::get_state_proof(
    //         ctx,
    //         "account",
    //         M::Serde::serialize(&non_existent_eth_address),
    //     )
    //     .unwrap();

    //     // Verify proof of non-existence.
    //     proof
    //         .verify_non_membership::<EthAddress, M>("account", non_existent_eth_address,
    // state_root)         .unwrap();
    // });

    // Check that all nodes are present in the state tree.
    for node_public_key in node_public_keys.iter() {
        query.run(|ctx| {
            let node_index = {
                let node_pub_key_to_index_table =
                    ctx.get_table::<NodePublicKey, NodeIndex>("pub_key_to_index");
                let value = node_pub_key_to_index_table.get(node_public_key).unwrap();

                // Generate proof of existence.
                let proof = M::get_state_proof(
                    ctx,
                    "pub_key_to_index",
                    M::Serde::serialize(&node_public_key),
                )
                .unwrap();

                // Verify proof of existence.
                proof
                    .verify_membership::<NodePublicKey, NodeIndex, M>(
                        "pub_key_to_index",
                        node_public_key,
                        value,
                        state_root,
                    )
                    .unwrap();

                value
            };

            // Check that each node has a corresponding node info with proof of existence.
            let nodes_info_table = ctx.get_table::<NodeIndex, NodeInfo>("node");
            let value = nodes_info_table.get(node_index).unwrap();

            // Generate proof of existence.
            let proof = M::get_state_proof(ctx, "node", M::Serde::serialize(&node_index)).unwrap();

            // Verify proof of existence.
            proof
                .verify_membership::<NodeIndex, NodeInfo, M>("node", node_index, value, state_root)
                .unwrap();
        });
    }
}
