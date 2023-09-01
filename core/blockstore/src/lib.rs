pub mod blockstore;
pub mod config;
pub mod put;
mod store;

#[cfg(test)]
mod tests {
    #![allow(unused)]

    use blake3_tree::blake3::tree::{BlockHasher, HashTree, HashTreeBuilder};
    use blake3_tree::ProofBuf;
    use lightning_interfaces::infu_collection::Collection;
    use lightning_interfaces::types::{CompressionAlgoSet, CompressionAlgorithm};
    use lightning_interfaces::{partial, Blake3Hash, BlockStoreInterface, IncrementalPutInterface};
    use tokio::test;

    use crate::blockstore::{Blockstore, BLOCK_SIZE};
    use crate::config::Config;

    partial!(TestBinding {
        BlockStoreInterface = Blockstore<Self>;
    });

    fn create_content() -> Vec<u8> {
        (0..4)
            .map(|i| Vec::from([i; BLOCK_SIZE]))
            .flat_map(|a| a.into_iter())
            .collect()
    }

    fn hash_tree(content: &[u8]) -> HashTree {
        let mut tree_builder = HashTreeBuilder::new();
        tree_builder.update(content);
        tree_builder.finalize()
    }

    fn new_proof(tree: &[[u8; 32]], block: usize) -> ProofBuf {
        if block > 0 {
            ProofBuf::resume(tree, block)
        } else {
            ProofBuf::new(tree, block)
        }
    }

    // #[test]
    // async fn test_put() {
    //     // Given: some content.
    //     let content = create_content();
    //     // Given: a block store.
    //     let blockstore = Blockstore::<TestBinding>::init(Config::default()).unwrap();
    //     // When: we create a putter and write some content.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(content.as_slice(), CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     // Then: the putter returns the appropriate root hash.
    //     let hash_tree = hash_tree(content.as_slice());
    //     let root = putter.finalize().await.unwrap();
    //     assert_eq!(root, Blake3Hash::from(hash_tree.hash));
    // }
    //
    // #[test]
    // async fn test_put_verify() {
    //     // Given: some content.
    //     let content = create_content();
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(content.as_slice(), CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // Given: the full tree.
    //     let hash_tree = hash_tree(content.as_slice());
    //     // When: we put the content by block and feed the proof to verify it.
    //     let mut putter = blockstore.put(Some(Blake3Hash::from(hash_tree.hash)));
    //     for (i, block) in content.chunks(BLOCK_SIZE).enumerate() {
    //         let proof = new_proof(&hash_tree.tree, i);
    //         putter.feed_proof(proof.as_slice()).unwrap();
    //         putter
    //             .write(block, CompressionAlgorithm::Uncompressed)
    //             .unwrap();
    //     }
    //     // Then: the putter returns the appropriate root hash and no errors.
    //     let root = putter.finalize().await.unwrap();
    //     assert_eq!(root, Blake3Hash::from(hash_tree.hash));
    // }
    //
    // #[test]
    // async fn test_put_verify_invalid_content() {
    //     // Given: some content.
    //     let mut content = create_content();
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store and feed the proof to verify it.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(content.as_slice(), CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // Given: the full tree.
    //     let hash_tree = hash_tree(content.as_slice());
    //     // Given: make a change to the content.
    //     content[10] = 69;
    //     // When: we put a block with modified content and feed the proof to verify it.
    //     let mut putter = blockstore.put(Some(Blake3Hash::from(hash_tree.hash)));
    //     let mut blocks = content.chunks(BLOCK_SIZE);
    //     let proof = ProofBuf::new(&hash_tree.tree, 0);
    //     putter.feed_proof(proof.as_slice()).unwrap();
    //     let write_result = putter.write(blocks.next().unwrap(),
    // CompressionAlgorithm::Uncompressed);     // Then: the putter returns the appropriate
    // errors.     assert!(write_result.is_err());
    // }
    //
    // #[test]
    // async fn test_get() {
    //     // Given: some content.
    //     let content = create_content();
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(content.as_slice(), CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // When: we query the block store for our blocks using their hashes.
    //     for (count, chunk) in content.chunks(BLOCK_SIZE).enumerate() {
    //         let mut block = BlockHasher::new();
    //         block.set_block(count);
    //         block.update(chunk);
    //         let hash = block.finalize(false);
    //         let content_from_store = blockstore
    //             .get(count as u32, &hash, CompressionAlgoSet::new())
    //             .await
    //             .unwrap();
    //         // Then: we get our content as expected.
    //         assert_eq!(content_from_store.content, chunk);
    //     }
    //     // Then: our tree is stored as expected.
    //     let hash_tree = hash_tree(content.as_slice());
    //     assert_eq!(
    //         hash_tree.tree,
    //         blockstore
    //             .get_tree(&Blake3Hash::from(hash_tree.hash))
    //             .await
    //             .unwrap()
    //             .0
    //     )
    // }
    //
    // #[test]
    // async fn test_put_verify_one_chunk() {
    //     // Given: some content.
    //     let content = [0; BLOCK_SIZE];
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // Given: the full tree.
    //     let hash_tree = hash_tree(&content);
    //     // When: we put one block and feed the proof to verify it.
    //     let mut putter = blockstore.put(Some(Blake3Hash::from(hash_tree.hash)));
    //     let proof = new_proof(&hash_tree.tree, 0);
    //     putter.feed_proof(proof.as_slice()).unwrap();
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     // Then: the putter returns the appropriate root hash and no errors.
    //     let root = putter.finalize().await.unwrap();
    //     assert_eq!(root, Blake3Hash::from(hash_tree.hash));
    // }
    //
    // #[test]
    // async fn test_put_verify_one_chunk_small() {
    //     // Given: some content.
    //     let content = [0; 256];
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // Given: the full tree.
    //     let hash_tree = hash_tree(&content);
    //     // When: we put one block and feed the proof to verify it.
    //     let mut putter = blockstore.put(Some(Blake3Hash::from(hash_tree.hash)));
    //     let proof = new_proof(&hash_tree.tree, 0);
    //     putter.feed_proof(proof.as_slice()).unwrap();
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     // Then: the putter returns the appropriate root hash and no errors.
    //     let root = putter.finalize().await.unwrap();
    //     assert_eq!(root, Blake3Hash::from(hash_tree.hash));
    // }
    //
    // #[test]
    // async fn test_put_verify_one_chunk_and_a_half() {
    //     // Given: one chunk and another chunk smaller than a Blake3 chunk.
    //     let content = vec![vec![0; 256 * 1024], vec![1; 256]]
    //         .into_iter()
    //         .flatten()
    //         .collect::<Vec<_>>();
    //     // Given: a block store.
    //     let blockstore = MemoryBlockStore::<TestBinding>::init(Config {}).unwrap();
    //     // Given: we put the content in the block store.
    //     let mut putter = blockstore.put(None);
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     putter.finalize().await.unwrap();
    //     // Given: the full tree.
    //     let hash_tree = hash_tree(&content);
    //     // When: we put the content by block and feed the proof to verify it.
    //     for (count, chunk) in content.chunks(BLOCK_SIZE).enumerate() {
    //         let mut block = BlockHasher::new();
    //         block.set_block(count);
    //         block.update(chunk);
    //         let hash = block.finalize(false);
    //         let content_from_store = blockstore
    //             .get(count as u32, &hash, CompressionAlgoSet::new())
    //             .await
    //             .unwrap();
    //         // Then: we get our content as expected.
    //         assert_eq!(content_from_store.content, chunk);
    //     }
    //     let mut putter = blockstore.put(Some(Blake3Hash::from(hash_tree.hash)));
    //     let proof = new_proof(&hash_tree.tree, 0);
    //     putter.feed_proof(proof.as_slice()).unwrap();
    //     putter
    //         .write(&content, CompressionAlgorithm::Uncompressed)
    //         .unwrap();
    //     // Then: the putter returns the appropriate root hash and no errors.
    //     let root = putter.finalize().await.unwrap();
    //     assert_eq!(root, Blake3Hash::from(hash_tree.hash));
    // }

    #[test]
    async fn test_put_get_fs() {
        // Given: some content.
        let content = create_content();
        let path = std::env::temp_dir().join("lightning-blockstore-test");
        // Given: a block store.
        let blockstore = Blockstore::<TestBinding>::init(Config::default()).unwrap();

        let test = || async {
            // When: we create a putter and write some content.
            let mut putter = blockstore.put(None);
            putter
                .write(content.as_slice(), CompressionAlgorithm::Uncompressed)
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;

            // Then: the putter returns the appropriate root hash.
            let root = putter
                .finalize()
                .await
                .map_err(|e| anyhow::anyhow!("{e:?}"))?;
            let tree = hash_tree(content.as_slice());
            if root != Blake3Hash::from(tree.hash) {
                anyhow::bail!("invalid root hash");
            }

            // When: we query the block store for our blocks using their hashes.
            for (count, chunk) in content.chunks(BLOCK_SIZE).enumerate() {
                let mut block = BlockHasher::new();
                block.set_block(count);
                block.update(chunk);
                let hash = block.finalize(false);
                let content_from_store = blockstore
                    .get(count as u32, &hash, CompressionAlgoSet::new())
                    .await
                    .ok_or_else(|| anyhow::anyhow!("failed to get chunk"))?;

                // Then: we get our content as expected.
                if content_from_store.content != chunk {
                    anyhow::bail!("chunk is invalid");
                }
            }

            // Then: our tree is stored as expected.
            let tree = hash_tree(content.as_slice());
            if tree.tree
                != blockstore
                    .get_tree(&Blake3Hash::from(tree.hash))
                    .await
                    .ok_or_else(|| anyhow::anyhow!("failed to get tree"))?
                    .0
            {
                anyhow::bail!("tree is invalid");
            }
            Ok(())
        };

        let root = test().await;

        if path.exists() {
            std::fs::remove_dir_all(path).unwrap();
        }

        root.unwrap();
    }
}
