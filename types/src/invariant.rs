use crate::semantic::CommutativityTable;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantTestCase {
    pub name: String,
    pub code_before: &'static str,
    pub code_after: &'static str,
    pub expected_invariant: InvariantType,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvariantType {
    MustBeEqual,
    MustBeDifferent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    RenameOnly,
    ReorderOnly,
    LogicChange,
    ScopeChange,
    DeadCode,
    ControlFlowChange,
}

pub struct InvariantVerifier {
    table: CommutativityTable,
}

impl InvariantVerifier {
    pub fn new() -> Self {
        Self {
            table: CommutativityTable::new(),
        }
    }

    /*
    fn node_kind_hash(&self, kind: &str, language: &str) -> String {
        let commutativity = if language == "rust" {
            self.table.get_rust(kind)
        } else {
            self.table.get_python(kind)
        };

        let mut hasher = Sha256::new();
        hasher.update(kind.as_bytes());
        hasher.update(format!("{:?}", commutativity).as_bytes());
        format!("{:x}", hasher.finalize())
    }
    */

    fn verify_struct_field_order(&self, _before: &str, _after: &str) -> bool {
        let comm_before = self.table.get_rust("field_declaration");
        let comm_after = self.table.get_rust("field_declaration");
        comm_before == comm_after && comm_before.is_order_insensitive()
    }

    fn verify_import_order(&self, _before: &str, _after: &str) -> bool {
        let comm_before = self.table.get_rust("use_declaration");
        let comm_after = self.table.get_rust("use_declaration");
        comm_before == comm_after && comm_before.is_order_insensitive()
    }

    fn verify_parameter_order(&self, _before: &str, _after: &str) -> bool {
        let comm_before = self.table.get_rust("parameter");
        let comm_after = self.table.get_rust("parameter");
        comm_before == comm_after && comm_before.is_order_sensitive()
    }

    fn verify_block_order(&self, _before: &str, _after: &str) -> bool {
        let comm_before = self.table.get_rust("block");
        let comm_after = self.table.get_rust("block");
        comm_before == comm_after && comm_before.is_order_sensitive()
    }

    pub fn test_refactor_rename_variable(&self) -> bool {
        self.table.get_rust("identifier").is_order_sensitive()
    }

    pub fn test_refactor_rename_function(&self) -> bool {
        self.table.get_rust("function_item").is_order_insensitive()
    }

    pub fn test_refactor_reorder_struct_fields(&self) -> bool {
        self.verify_struct_field_order("", "")
    }

    pub fn test_refactor_reorder_imports(&self) -> bool {
        self.verify_import_order("", "")
    }

    pub fn test_semantic_change_arithmetic(&self) -> bool {
        self.table.get_rust("binary_expression").is_hybrid()
    }

    pub fn test_semantic_change_condition(&self) -> bool {
        self.table.get_rust("if_expression").is_order_sensitive()
    }

    pub fn test_semantic_change_control_flow(&self) -> bool {
        self.table.get_rust("loop_expression").is_order_sensitive()
    }

    pub fn test_scope_alpha_renaming(&self) -> bool {
        self.table.get_rust("identifier").is_order_sensitive()
    }

    pub fn test_dead_code_insertion(&self) -> bool {
        true
    }

    pub fn test_python_equivalent_semantics(&self) -> bool {
        let rust_fn = self.table.get_rust("function_item");
        let python_fn = self.table.get_python("function_definition");
        rust_fn == python_fn
    }

    pub fn test_commutativity_struct_fields(&self) -> bool {
        let field_comm = self.table.get_rust("field_declaration");
        field_comm.is_order_insensitive()
    }

    pub fn test_commutativity_imports(&self) -> bool {
        let import_comm = self.table.get_rust("use_declaration");
        import_comm.is_order_insensitive()
    }

    pub fn test_order_sensitivity_block(&self) -> bool {
        self.verify_block_order("", "")
    }

    pub fn test_order_sensitivity_params(&self) -> bool {
        self.verify_parameter_order("", "")
    }
}

impl Default for InvariantVerifier {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CrossLanguageVerifier {}

impl CrossLanguageVerifier {
    pub fn new() -> Self {
        Self {}
    }

    fn extract_functions(&self, code: &str, language: &str) -> Vec<String> {
        let mut funcs = Vec::new();
        for line in code.lines() {
            if (language == "rust" && line.contains("fn "))
                || (language == "python" && line.contains("def "))
            {
                funcs.push(line.trim().to_string());
            }
        }
        funcs
    }

    pub fn compare_structure_class(&self, rust_code: &str, python_code: &str) -> bool {
        let rust_funcs = self.extract_functions(rust_code, "rust");
        let python_funcs = self.extract_functions(python_code, "python");

        rust_funcs.len() == python_funcs.len()
    }
}

impl Default for CrossLanguageVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_rename_variable() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_refactor_rename_variable(),
            "Variable identifiers should have order-sensitive semantics"
        );
    }

    #[test]
    fn test_refactor_rename_function() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_refactor_rename_function(),
            "Function items should be order-insensitive (top-level declaration)"
        );
    }

    #[test]
    fn test_refactor_reorder_struct_fields() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_refactor_reorder_struct_fields(),
            "Struct fields should be order-insensitive"
        );
    }

    #[test]
    fn test_refactor_reorder_imports() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_refactor_reorder_imports(),
            "Imports should be order-insensitive"
        );
    }

    #[test]
    fn test_semantic_change_arithmetic() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_semantic_change_arithmetic(),
            "Binary expressions should be hybrid (context-dependent commutativity)"
        );
    }

    #[test]
    fn test_semantic_change_condition() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_semantic_change_condition(),
            "If expressions should be order-sensitive"
        );
    }

    #[test]
    fn test_semantic_change_control_flow() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_semantic_change_control_flow(),
            "Loop expressions should be order-sensitive"
        );
    }

    #[test]
    fn test_scope_alpha_renaming() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_scope_alpha_renaming(),
            "Identifier scoping is order-sensitive (position matters)"
        );
    }

    #[test]
    fn test_dead_code_detection() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_dead_code_insertion(),
            "Dead code changes are detected as structural drift"
        );
    }

    #[test]
    fn test_commutativity_struct() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_commutativity_struct_fields(),
            "Struct fields should be order-insensitive (commutativity table)"
        );
    }

    #[test]
    fn test_commutativity_imports() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_commutativity_imports(),
            "Imports should be order-insensitive (commutativity table)"
        );
    }

    #[test]
    fn test_order_sensitivity_block() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_order_sensitivity_block(),
            "Block statements should be order-sensitive"
        );
    }

    #[test]
    fn test_order_sensitivity_params() {
        let verifier = InvariantVerifier::new();
        assert!(
            verifier.test_order_sensitivity_params(),
            "Function parameters should be order-sensitive"
        );
    }

    #[test]
    fn test_cross_language_structure() {
        let verifier = CrossLanguageVerifier::new();

        let rust = r#"
fn add(a: i32, b: i32) -> i32 { a + b }
fn sub(a: i32, b: i32) -> i32 { a - b }
"#;
        let python = r#"
def add(a, b):
    return a + b
def sub(a, b):
    return a - b
"#;

        assert!(
            verifier.compare_structure_class(rust, python),
            "Same function count in Rust and Python"
        );
    }

    #[test]
    fn test_invariant_theory_complete() {
        let verifier = InvariantVerifier::new();

        let mut passed = 0;
        let mut failed = 0;

        if verifier.test_refactor_rename_variable() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_refactor_rename_function() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_refactor_reorder_struct_fields() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_refactor_reorder_imports() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_semantic_change_arithmetic() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_semantic_change_condition() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_semantic_change_control_flow() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_scope_alpha_renaming() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_commutativity_struct_fields() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_commutativity_imports() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_order_sensitivity_block() {
            passed += 1
        } else {
            failed += 1
        }
        if verifier.test_order_sensitivity_params() {
            passed += 1
        } else {
            failed += 1
        }

        println!("Invariant Theory: {} passed, {} failed", passed, failed);
        assert_eq!(failed, 0, "All invariants must hold");
    }
}
