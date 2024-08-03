const SPARSE_MERKLE_PLACEHOLDER_HASH: [u8; 32] = *b"SPARSE_MERKLE_PLACEHOLDER_HASH__";
const LEAF_DOMAIN_SEPARATOR: &[u8] = b"JMT::LeafNode";
const INTERNAL_DOMAIN_SEPARATOR: &[u8] = b"JMT::IntrnalNode";

/// Return the ICS23 proof spec for the merklize provider, customized to the specific hasher.
pub fn ics23_proof_spec(hash_op: ics23::HashOp) -> ics23::ProofSpec {
    ics23::ProofSpec {
        leaf_spec: Some(ics23::LeafOp {
            hash: hash_op.into(),
            prehash_key: hash_op.into(),
            prehash_value: hash_op.into(),
            length: ics23::LengthOp::NoPrefix.into(),
            prefix: LEAF_DOMAIN_SEPARATOR.to_vec(),
        }),
        inner_spec: Some(ics23::InnerSpec {
            hash: hash_op.into(),
            child_order: vec![0, 1],
            min_prefix_length: INTERNAL_DOMAIN_SEPARATOR.len() as i32,
            max_prefix_length: INTERNAL_DOMAIN_SEPARATOR.len() as i32,
            child_size: 32,
            empty_child: SPARSE_MERKLE_PLACEHOLDER_HASH.to_vec(),
        }),
        min_depth: 0,
        max_depth: 64,
        prehash_key_before_comparison: true,
    }
}
