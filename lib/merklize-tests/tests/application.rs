use atomo::{DefaultSerdeBackend, SerdeBackend, UpdatePerm};
use fleek_crypto::{AccountOwnerSecretKey, EthAddress, NodePublicKey, SecretKey};
use lightning_application::env::Env;
use lightning_application::storage::AtomoStorage;
use lightning_types::{AccountInfo, NodeIndex, NodeInfo};
use merklize::hashers::blake3::Blake3Hasher;
use merklize::hashers::keccak::KeccakHasher;
use merklize::hashers::sha2::Sha256Hasher;
use merklize::providers::jmt::JmtMerklizeProvider;
use merklize::MerklizeProvider;
use merklize_tests::application::{create_rocksdb_env, new_complex_block, DummyPutter};
use tempfile::tempdir;

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

async fn test_application<M>(env: &mut Env<UpdatePerm, AtomoStorage, M>)
where
    M: MerklizeProvider<Storage = AtomoStorage, Serde = DefaultSerdeBackend>,
{
    let _ = tracing_subscriber::fmt::try_init();

    let (block, _stake_amount, eth_addresses, node_public_keys) = new_complex_block();

    env.run(block.clone(), || DummyPutter {}).await;

    let query = env.inner.query();

    let state_root = query.get_state_root().unwrap();

    // Check that all accounts are present in the state tree.
    for eth_address in eth_addresses.iter() {
        query.run(|ctx| {
            let accounts_table = ctx.get_table::<EthAddress, AccountInfo>("account");
            let expected = accounts_table.get(eth_address).unwrap();

            // Generate proof of existence.
            let ctx = M::context(ctx);
            let (value, proof) = ctx
                .get_state_proof("account", M::Serde::serialize(&eth_address))
                .unwrap();

            // Check that values match.
            assert!(value.is_some());
            let actual = M::Serde::deserialize::<AccountInfo>(value.unwrap().as_slice());
            assert_eq!(actual, expected);

            // Verify proof of existence.
            assert!(proof.verify_membership::<EthAddress, AccountInfo, M>(
                "account",
                eth_address,
                actual,
                state_root
            ));
        });
    }

    // Check proof of non-existence for an account that does not exist.
    // TODO(snormore): Figure out why this test fails.
    let non_existent_eth_address: EthAddress = {
        let secret_key = AccountOwnerSecretKey::generate();
        let public_key = secret_key.to_pk();
        public_key.into()
    };
    query.run(|ctx| {
        let accounts_table = ctx.get_table::<EthAddress, AccountInfo>("account");

        // Verify that the account does not exist.
        assert!(accounts_table.get(non_existent_eth_address).is_none());

        // Generate proof of non-existence.
        let ctx = M::context(ctx);
        let (value, proof) = ctx
            .get_state_proof("account", M::Serde::serialize(&non_existent_eth_address))
            .unwrap();
        assert!(value.is_none());

        println!("state_root: {:?}", state_root);
        println!("value: {:?}", value);
        // println!("proof: {:?}", proof);

        // Verify proof of non-existence.
        assert!(proof.verify_non_membership::<EthAddress, M>(
            "account",
            non_existent_eth_address,
            query.get_state_root().unwrap()
        ));
    });

    // Check that all nodes are present in the state tree.
    for node_public_key in node_public_keys.iter() {
        query.run(|ctx| {
            let node_index = {
                let node_pub_key_to_index_table =
                    ctx.get_table::<NodePublicKey, NodeIndex>("pub_key_to_index");
                let expected = node_pub_key_to_index_table.get(node_public_key).unwrap();

                // Generate proof of existence.
                let ctx = M::context(ctx);
                let (value, proof) = ctx
                    .get_state_proof("pub_key_to_index", M::Serde::serialize(&node_public_key))
                    .unwrap();

                // Check that values match.
                assert!(value.is_some());
                let actual = M::Serde::deserialize::<NodeIndex>(value.unwrap().as_slice());
                assert_eq!(actual, expected);

                // Verify proof of existence.
                assert!(proof.verify_membership::<NodePublicKey, NodeIndex, M>(
                    "pub_key_to_index",
                    node_public_key,
                    actual,
                    state_root
                ));

                actual
            };

            // Check that each node has a corresponding node info with proof of existence.
            let nodes_info_table = ctx.get_table::<NodeIndex, NodeInfo>("node");
            let expected = nodes_info_table.get(node_index).unwrap();

            // Generate proof of existence.
            let ctx = M::context(ctx);
            let (value, proof) = ctx
                .get_state_proof("node", M::Serde::serialize(&node_index))
                .unwrap();

            // Check that values match.
            assert!(value.is_some());
            let actual = M::Serde::deserialize::<NodeInfo>(value.unwrap().as_slice());
            assert_eq!(actual, expected);

            // Verify proof of existence.
            assert!(proof.verify_membership::<NodeIndex, NodeInfo, M>(
                "node", node_index, actual, state_root
            ));
        });
    }
}
