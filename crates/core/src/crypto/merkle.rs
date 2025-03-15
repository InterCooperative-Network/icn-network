//! Merkle Tree implementation
//!
//! This module provides a Merkle tree implementation for efficient verification
//! of data integrity in a distributed system.

use std::collections::VecDeque;
use super::{Hash, sha256, CryptoResult, CryptoError};

/// A Merkle Tree data structure
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// The root hash of the tree
    root: Hash,
    /// The leaf nodes (data hashes)
    leaves: Vec<Hash>,
    /// The internal nodes
    nodes: Vec<Vec<Hash>>,
}

impl MerkleTree {
    /// Create a new Merkle tree from a list of data items
    pub fn new<T: AsRef<[u8]>>(data: &[T]) -> Self {
        if data.is_empty() {
            // Empty tree has a zero hash
            return Self {
                root: Hash(vec![0; 32]),
                leaves: Vec::new(),
                nodes: Vec::new(),
            };
        }
        
        // Hash each data item to create leaf nodes
        let leaves: Vec<Hash> = data.iter()
            .map(|d| sha256(d.as_ref()))
            .collect();
        
        let mut nodes = Vec::new();
        let mut current_level = leaves.clone();
        
        // Build the tree bottom-up
        while current_level.len() > 1 {
            nodes.push(current_level.clone());
            let mut next_level = Vec::new();
            
            for chunk in current_level.chunks(2) {
                if chunk.len() == 2 {
                    // Combine two hashes
                    let mut combined = chunk[0].as_bytes().to_vec();
                    combined.extend_from_slice(chunk[1].as_bytes());
                    next_level.push(sha256(&combined));
                } else {
                    // Odd number of nodes, promote the single node
                    next_level.push(chunk[0].clone());
                }
            }
            
            current_level = next_level;
        }
        
        // The root is the last node computed
        let root = current_level[0].clone();
        
        Self {
            root,
            leaves,
            nodes,
        }
    }
    
    /// Get the root hash of the tree
    pub fn root(&self) -> &Hash {
        &self.root
    }
    
    /// Get the number of leaves in the tree
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }
    
    /// Get the hash of a specific leaf
    pub fn leaf_hash(&self, index: usize) -> Option<&Hash> {
        self.leaves.get(index)
    }
    
    /// Generate a proof for a specific leaf
    pub fn generate_proof(&self, leaf_index: usize) -> CryptoResult<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return Err(CryptoError::HashingError(
                format!("Leaf index {} out of bounds (0-{})", leaf_index, self.leaves.len() - 1)
            ));
        }
        
        let mut proof = Vec::new();
        let mut current_idx = leaf_index;
        
        for level in 0..self.nodes.len() {
            let level_nodes = &self.nodes[level];
            let is_right = current_idx % 2 == 0;
            let sibling_idx = if is_right { current_idx + 1 } else { current_idx - 1 };
            
            if sibling_idx < level_nodes.len() {
                proof.push(ProofNode {
                    hash: level_nodes[sibling_idx].clone(),
                    is_left: !is_right,
                });
            }
            
            current_idx /= 2;
        }
        
        Ok(MerkleProof {
            leaf_hash: self.leaves[leaf_index].clone(),
            proof,
            root: self.root.clone(),
        })
    }
    
    /// Verify that data belongs to the tree at a given index
    pub fn verify(&self, data: &[u8], index: usize) -> CryptoResult<bool> {
        if index >= self.leaves.len() {
            return Err(CryptoError::HashingError(
                format!("Leaf index {} out of bounds (0-{})", index, self.leaves.len() - 1)
            ));
        }
        
        let data_hash = sha256(data);
        let leaf_hash = &self.leaves[index];
        
        if data_hash != *leaf_hash {
            return Ok(false);
        }
        
        let proof = self.generate_proof(index)?;
        proof.verify(&data_hash)
    }
}

/// A node in a Merkle proof
#[derive(Debug, Clone)]
pub struct ProofNode {
    /// The hash of the node
    pub hash: Hash,
    /// Whether this node is to the left of the path
    pub is_left: bool,
}

/// A Merkle proof for a specific leaf
#[derive(Debug, Clone)]
pub struct MerkleProof {
    /// The hash of the leaf being proven
    pub leaf_hash: Hash,
    /// The proof nodes, from bottom to top
    pub proof: Vec<ProofNode>,
    /// The expected root hash
    pub root: Hash,
}

impl MerkleProof {
    /// Verify a proof against the expected root
    pub fn verify(&self, leaf_hash: &Hash) -> CryptoResult<bool> {
        if *leaf_hash != self.leaf_hash {
            return Ok(false);
        }
        
        let mut current_hash = leaf_hash.clone();
        
        for node in &self.proof {
            let mut combined = Vec::new();
            
            if node.is_left {
                combined.extend_from_slice(node.hash.as_bytes());
                combined.extend_from_slice(current_hash.as_bytes());
            } else {
                combined.extend_from_slice(current_hash.as_bytes());
                combined.extend_from_slice(node.hash.as_bytes());
            }
            
            current_hash = sha256(&combined);
        }
        
        Ok(current_hash == self.root)
    }
} 