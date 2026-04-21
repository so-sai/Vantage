use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::semantic::{AlgebraResolver, CafContext, Commutativity, ScopeContext};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CafNode {
    pub kind: String,
    pub semantic_id: Option<String>,
    pub children: Vec<CafHash>,
    pub commutativity: Commutativity,
    pub byte_start: usize,
    pub byte_end: usize,
    pub parent_id: Option<crate::node_id::NodeId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CafHash {
    pub algorithm: String,
    pub value: String,
}

impl CafHash {
    pub fn sha256(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self {
            algorithm: "sha256".to_string(),
            value: format!("{:x}", result),
        }
    }

    pub fn combine(&self, other: &CafHash) -> Self {
        let combined = format!("{}:{}", self.value, other.value);
        Self::sha256(combined.as_bytes())
    }
}

pub struct CafBuilder<'a> {
    resolver: &'a dyn AlgebraResolver,
    language: &'a str,
    scope: CafContext,
}

impl<'a> CafBuilder<'a> {
    pub fn new(resolver: &'a dyn AlgebraResolver, language: &'a str) -> Self {
        Self {
            resolver,
            language,
            scope: CafContext::new(),
        }
    }

    pub fn with_scope(mut self, scope: CafContext) -> Self {
        self.scope = scope;
        self
    }

    pub fn build(
        &mut self,
        kind: &str,
        child_hashes: Vec<CafHash>,
        start: usize,
        end: usize,
        parent_id: Option<crate::node_id::NodeId>,
    ) -> (CafNode, CafHash) {
        let context = self.scope.scope_context();
        let commutativity = self.resolver.resolve(kind, self.language, &context);

        let canonical_hash = self.compute_caf_hash(&commutativity, &child_hashes);

        let node = CafNode {
            kind: kind.to_string(),
            semantic_id: None,
            children: child_hashes,
            commutativity,
            byte_start: start,
            byte_end: end,
            parent_id,
        };

        (node, canonical_hash)
    }

    fn compute_caf_hash(&self, commutativity: &Commutativity, child_hashes: &[CafHash]) -> CafHash {
        if child_hashes.is_empty() {
            return CafHash::sha256(&[]);
        }

        let ordered: Vec<CafHash> = match commutativity {
            Commutativity::OrderSensitive => child_hashes.to_vec(),
            Commutativity::OrderInsensitive | Commutativity::Hybrid => {
                let mut sorted = child_hashes.to_vec();
                sorted.sort_by(|a, b| a.value.cmp(&b.value));
                sorted
            }
        };

        let mut combined = String::new();
        for h in &ordered {
            combined.push_str(&h.value);
            combined.push(':');
        }

        CafHash::sha256(combined.as_bytes())
    }

    pub fn declare_var(&mut self, name: &str) -> String {
        self.scope.declare_var(name)
    }

    pub fn declare_func(&mut self, name: &str) -> String {
        self.scope.declare_func(name)
    }

    pub fn declare_type(&mut self, name: &str) -> String {
        self.scope.declare_type(name)
    }

    pub fn resolve(&self, name: &str) -> Option<String> {
        self.scope.resolve(name)
    }

    pub fn push_scope(&mut self) {
        self.scope.push_scope();
    }

    pub fn pop_scope(&mut self) {
        self.scope.pop_scope();
    }

    pub fn scope_depth(&self) -> usize {
        self.scope.depth()
    }
}

pub struct CafDiffer<'a> {
    resolver: &'a dyn AlgebraResolver,
}

impl<'a> CafDiffer<'a> {
    pub fn new(resolver: &'a dyn AlgebraResolver) -> Self {
        Self { resolver }
    }

    pub fn diff(&self, left: &CafNode, right: &CafNode, language: &str) -> CafDiffResult {
        let context = ScopeContext::new();

        if left.kind != right.kind {
            return CafDiffResult {
                changed: true,
                reason: CafDiffReason::KindMismatch(left.kind.clone(), right.kind.clone()),
                left_hash: left.children.clone(),
                right_hash: right.children.clone(),
            };
        }

        let left_hash = self.compute_hash(left, language, &context);
        let right_hash = self.compute_hash(right, language, &context);

        CafDiffResult {
            changed: left_hash != right_hash,
            reason: CafDiffReason::ContentChanged,
            left_hash: left.children.clone(),
            right_hash: right.children.clone(),
        }
    }

    fn compute_hash(&self, node: &CafNode, language: &str, context: &ScopeContext) -> CafHash {
        let commutativity = self.resolver.resolve(&node.kind, language, context);

        let ordered: Vec<CafHash> = match commutativity {
            Commutativity::OrderSensitive => node.children.clone(),
            Commutativity::OrderInsensitive | Commutativity::Hybrid => {
                let mut sorted = node.children.clone();
                sorted.sort_by(|a, b| a.value.cmp(&b.value));
                sorted
            }
        };

        let mut combined = String::new();
        for h in &ordered {
            combined.push_str(&h.value);
            combined.push(':');
        }

        CafHash::sha256(combined.as_bytes())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CafDiffResult {
    pub changed: bool,
    pub reason: CafDiffReason,
    pub left_hash: Vec<CafHash>,
    pub right_hash: Vec<CafHash>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CafDiffReason {
    KindMismatch(String, String),
    ContentChanged,
    StructureChanged,
    CommutativityChanged,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::CommutativityTable;

    struct TestResolver {
        table: CommutativityTable,
    }

    impl AlgebraResolver for TestResolver {
        fn resolve(
            &self,
            node_kind: &str,
            language: &str,
            _context: &ScopeContext,
        ) -> Commutativity {
            if language == "rust" {
                self.table.get_rust(node_kind)
            } else {
                self.table.get_python(node_kind)
            }
        }
    }

    #[test]
    fn test_caf_hash_order_insensitive() {
        let resolver = TestResolver {
            table: CommutativityTable::new(),
        };
        let mut builder = CafBuilder::new(&resolver, "rust");

        let child1 = CafHash::sha256(b"a");
        let child2 = CafHash::sha256(b"b");

        let (_, hash) = builder.build(
            "struct_item",
            vec![child1.clone(), child2.clone()],
            0,
            10,
            None,
        );
        let (_, hash_reversed) =
            builder.build("struct_item", vec![child2, child1.clone()], 0, 10, None);

        assert_eq!(hash.value, hash_reversed.value);
    }

    #[test]
    fn test_caf_hash_order_sensitive() {
        let resolver = TestResolver {
            table: CommutativityTable::new(),
        };
        let mut builder = CafBuilder::new(&resolver, "rust");

        let child1 = CafHash::sha256(b"a");
        let child2 = CafHash::sha256(b"b");

        let (_, hash_ab) =
            builder.build("block", vec![child1.clone(), child2.clone()], 0, 10, None);
        let (_, hash_ba) = builder.build("block", vec![child2, child1], 0, 10, None);

        assert_ne!(hash_ab.value, hash_ba.value);
    }

    #[test]
    fn test_caf_differ_kind_mismatch() {
        let resolver = TestResolver {
            table: CommutativityTable::new(),
        };
        let differ = CafDiffer::new(&resolver);

        let left = CafNode {
            kind: "struct_item".to_string(),
            semantic_id: None,
            children: vec![],
            commutativity: Commutativity::OrderInsensitive,
            byte_start: 0,
            byte_end: 10,
            parent_id: None,
        };

        let right = CafNode {
            kind: "function_item".to_string(),
            semantic_id: None,
            children: vec![],
            commutativity: Commutativity::OrderSensitive,
            byte_start: 0,
            byte_end: 10,
            parent_id: None,
        };

        let result = differ.diff(&left, &right, "rust");

        assert!(result.changed);
        match result.reason {
            CafDiffReason::KindMismatch(_, _) => {}
            _ => panic!("Expected KindMismatch"),
        }
    }

    #[test]
    fn test_caf_builder_scope() {
        let resolver = TestResolver {
            table: CommutativityTable::new(),
        };
        let mut builder = CafBuilder::new(&resolver, "rust");

        let var_id = builder.declare_var("my_var");
        assert_eq!(var_id, "$V_0");

        builder.push_scope();
        let var_id_inner = builder.declare_var("my_var");
        assert_eq!(var_id_inner, "$V_1");

        builder.pop_scope();
        let resolved = builder.resolve("my_var");
        assert_eq!(resolved, Some("$V_0".to_string()));
    }
}
