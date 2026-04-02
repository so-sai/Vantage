use sha2::{Digest, Sha256};

pub struct Hasher;

impl Hasher {
    /// Computes a physical structural hash of the given content.
    /// This detects ANY physical change in the code block (whitespace, comments, etc.).
    pub fn structural_hash(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.trim().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Computes a normalized hash of the abstraction.
    /// Invariant to whitespace and formatting.
    pub fn normalized_hash(content: &str) -> String {
        let normalized = Self::normalize_pure_structure(content);
        let mut hasher = Sha256::new();
        hasher.update(normalized.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn normalize_pure_structure(content: &str) -> String {
        // Strip ALL whitespace for structural abstraction
        content.chars().filter(|c| !c.is_whitespace()).collect()
    }
}
