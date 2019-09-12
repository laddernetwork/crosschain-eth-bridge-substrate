/// 此文件程序仅针对于 ethereum 的 merkle patricia trie.

use kvdb::DBValue;
use ethereum_types::H256;
use ring::digest::{digest, SHA256};

pub fn hash(input: &[u8]) -> Vec<u8> {
    digest(&SHA256, input).as_ref().into()
}

/// 根据给定的trie leaf的hash和merkle proof，来验证该leaf是否存在
pub fn verify_merkle_proof(
    leaf: H256,
    branch: &[H256],
    depth: usize,
    index: usize,
    root: H256,
) -> bool {
    if branch.len() == depth {
        merkle_root_from_branch(leaf, branch, depth, index) == root
    } else {
        false
    }
}

/// 根据Merkle proof和trie leaf计算出trie root.
fn merkle_root_from_branch(leaf: H256, branch: &[H256], depth: usize, index: usize) -> H256 {
    assert_eq!(branch.len(), depth, "proof length should equal depth");

    let mut merkle_root = leaf.as_bytes().to_vec();

    for (i, &leaf) in branch.iter().enumerate().take(depth) {
        let ith_bit = (index >> i) & 0x01;
        if ith_bit == 1 {
            let input = concat(leaf.as_bytes().to_vec(), merkle_root);
            merkle_root = hash(&input);
        } else {
            let mut input = merkle_root;
            input.extend_from_slice(leaf.as_bytes());
            merkle_root = hash(&input);
        }
    }

    H256::from_slice(&merkle_root)
}

/// 连接两个Vec
fn concat(mut vec1: Vec<u8>, mut vec2: Vec<u8>) -> Vec<u8> {
    vec1.append(&mut vec2);
    vec1
}

#[cfg(test)]
mod verifyTests {
    use super::*;

    fn hash_concat(h1: H256, h2: H256) -> H256 {
        H256::from_slice(&hash(&concat(
            h1.as_bytes().to_vec(),
            h2.as_bytes().to_vec(),
        )))
    }

    #[test]
    fn verify_small_example() {
        // Construct a small merkle tree manually
        let leaf_b00 = H256::from([0xAA; 32]);
        let leaf_b01 = H256::from([0xBB; 32]);
        let leaf_b10 = H256::from([0xCC; 32]);
        let leaf_b11 = H256::from([0xDD; 32]);

        let node_b0x = hash_concat(leaf_b00, leaf_b01);
        let node_b1x = hash_concat(leaf_b10, leaf_b11);

        let root = hash_concat(node_b0x, node_b1x);

        // 测试一些merkle proof
        assert!(verify_merkle_proof(
            leaf_b00,
            &[leaf_b01, node_b1x],
            2,
            0b00,
            root
        ));
        assert!(verify_merkle_proof(
            leaf_b01,
            &[leaf_b00, node_b1x],
            2,
            0b01,
            root
        ));
        assert!(verify_merkle_proof(
            leaf_b10,
            &[leaf_b11, node_b0x],
            2,
            0b10,
            root
        ));
        assert!(verify_merkle_proof(
            leaf_b11,
            &[leaf_b10, node_b0x],
            2,
            0b11,
            root
        ));
        assert!(verify_merkle_proof(
            leaf_b11,
            &[leaf_b10],
            1,
            0b11,
            node_b1x
        ));

        // Ensure that incorrect proofs fail
        // Zero-length proof
        assert!(!verify_merkle_proof(leaf_b01, &[], 2, 0b01, root));
        // Proof in reverse order
        assert!(!verify_merkle_proof(
            leaf_b01,
            &[node_b1x, leaf_b00],
            2,
            0b01,
            root
        ));
        // Proof too short
        assert!(!verify_merkle_proof(leaf_b01, &[leaf_b00], 2, 0b01, root));
        // Wrong index
        assert!(!verify_merkle_proof(
            leaf_b01,
            &[leaf_b00, node_b1x],
            2,
            0b10,
            root
        ));
        // Wrong root
        assert!(!verify_merkle_proof(
            leaf_b01,
            &[leaf_b00, node_b1x],
            2,
            0b01,
            node_b1x
        ));
    }

    #[test]
    fn verify_zero_depth() {
        let leaf = H256::from([0xD6; 32]);
        let junk = H256::from([0xD7; 32]);
        assert!(verify_merkle_proof(leaf, &[], 0, 0, leaf));
        assert!(!verify_merkle_proof(leaf, &[], 0, 7, junk));
    }
}


/// Get merkle root of some hashed values - the input leaf nodes is expected to already be hashed
/// Outputs a `Vec<u8>` byte array of the merkle root given a set of leaf node values.
pub fn merkle_root(values: &[Vec<u8>]) -> Option<Vec<u8>> {
    let values_len = values.len();

    // check size of vector > 0 and ^ 2
    if values.is_empty() || !values_len.is_power_of_two() {
        return None;
    }

    // vector to store hashes
    // filled with 0 as placeholders
    let mut o: Vec<Vec<u8>> = vec![vec![0]; values_len];

    // append values to the end
    o.append(&mut values.to_vec());

    // traverse backwards as values are at the end
    // then fill placeholders with a hash of two leaf nodes
    for i in (0..values_len).rev() {
        let mut current_value: Vec<u8> = o[i * 2].clone();
        current_value.append(&mut o[i * 2 + 1].clone());

        o[i] = hash(&current_value[..]);
    }

    // the root hash will be at index 1
    Some(o[1].clone())
}

#[cfg(test)]
mod generateTests {
    use super::*;
    use ring::test;

    #[test]
    fn test_hashing() {
        let input: Vec<u8> = b"hello world".as_ref().into();

        let output = hash(input.as_ref());
        let expected_hex = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let expected: Vec<u8> = test::from_hex(expected_hex).unwrap();
        assert_eq!(expected, output);
    }

    #[test]
    fn test_merkle_root() {
        // hash the leaf nodes
        let mut input = vec![
            hash("a".as_bytes()),
            hash("b".as_bytes()),
            hash("c".as_bytes()),
            hash("d".as_bytes()),
        ];

        // generate a merkle tree and return the root
        let output = merkle_root(&input[..]);

        // create merkle root manually
        let mut leaf_1_2: Vec<u8> = input[0].clone(); // a
        leaf_1_2.append(&mut input[1].clone()); // b

        let mut leaf_3_4: Vec<u8> = input[2].clone(); // c
        leaf_3_4.append(&mut input[3].clone()); // d

        let node_1 = hash(&leaf_1_2[..]);
        let node_2 = hash(&leaf_3_4[..]);

        let mut root: Vec<u8> = node_1.clone(); // ab
        root.append(&mut node_2.clone()); // cd

        let expected = hash(&root[..]);

        assert_eq!(&expected[..], output.unwrap().as_slice());
    }
    #[test]
    fn test_empty_input_merkle_root() {
        let input = vec![];
        let output = merkle_root(&input[..]);
        assert_eq!(None, output);
    }
    #[test]
    fn test_odd_leaf_merkle_root() {
        let input = vec![
            hash("a".as_bytes()),
            hash("b".as_bytes()),
            hash("a".as_bytes()),
        ];
        let output = merkle_root(&input[..]);
        assert_eq!(None, output);
    }
}
