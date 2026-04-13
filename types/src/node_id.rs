use crate::collision::NodeFingerprint;
use crate::identity_anchor::IdentityAnchor;
use crate::semantic_role::SemanticRole;
use crate::version::HashVersion;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use xxhash_rust::xxh3::xxh3_64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u128);

impl Default for NodeId {
    fn default() -> Self {
        NodeId::INVALID
    }
}

/// Virtual Root for workspace-wide distinct identity.
/// Prevents root-level collisions across multiple files.
pub const DOMAIN_ROOT: NodeId = NodeId(0xDFD0_DFD0_DFD0_DFD0_DFD0_DFD0_DFD0_DFD0);

impl NodeId {
    pub const INVALID: NodeId = NodeId(0);

    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }

    /// The core identity formula for Vantage v1.2.4.
    /// NodeId = hash(parent_id, semantic_role, identity_anchor)
    pub fn generate(
        parent_id: Option<NodeId>,
        role: SemanticRole,
        anchor: &IdentityAnchor,
    ) -> (Self, NodeFingerprint) {
        let mut hasher = Sha256::new();

        // 0. Domain Separator
        hasher.update(b"NODEID_v1");

        // 1. Version Injection
        hasher.update(&HashVersion::CURRENT.semantic_role_version.to_le_bytes());
        hasher.update(&HashVersion::CURRENT.hash_algo_version.to_le_bytes());
        hasher.update(&HashVersion::CURRENT.canonicalization_version.to_le_bytes());

        // 2. Parent Salt
        match parent_id {
            Some(pid) => {
                hasher.update(b"PARENT");
                hasher.update(pid.0.to_le_bytes());
            }
            None => {
                hasher.update(b"ROOT_NODE");
            }
        }

        // 3. Salt with Semantic Role (ABI Stable)
        hasher.update(&(role as u16).to_le_bytes());

        // 4. Salt with Identity Anchor
        let anchor_bytes = anchor.to_stable_bytes();
        hasher.update(&anchor_bytes);

        let result = hasher.finalize();

        // Truncate to 128 bits
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&result[..16]);
        let primary = u128::from_le_bytes(bytes);

        // Secondary Fingerprint using xxHash
        let secondary = xxh3_64(&anchor_bytes);
        let secondary = if secondary == 0 { 1 } else { secondary };

        let id_val = if primary == 0 { 1 } else { primary }; // Prevent INVALID 0
        let node_id = NodeId(id_val);
        let fingerprint = NodeFingerprint::new(id_val, secondary);

        (node_id, fingerprint)
    }
}
