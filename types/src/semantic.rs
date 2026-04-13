use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Commutativity {
    OrderSensitive,
    OrderInsensitive,
    Hybrid,
}

impl Commutativity {
    #[inline]
    pub fn is_order_sensitive(&self) -> bool {
        matches!(self, Commutativity::OrderSensitive)
    }

    #[inline]
    pub fn is_order_insensitive(&self) -> bool {
        matches!(self, Commutativity::OrderInsensitive)
    }

    #[inline]
    pub fn is_hybrid(&self) -> bool {
        matches!(self, Commutativity::Hybrid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Constant,
    Variable,
    Field,
    Parameter,
    Block,
    Statement,
    Expression,
    Type,
    Import,
    Use,
    Call,
    Macro,
    Attribute,
    Guard,
    Loop,
    Match,
    Other,
}

#[derive(Debug, Error, Clone)]
pub enum SemanticError {
    #[error("Unknown tree-sitter node kind: {0}")]
    UnknownNodeKind(String),
    #[error("Scope overflow: maximum nesting depth exceeded")]
    ScopeOverflow,
    #[error("Identifier not found in scope: {0}")]
    UndefinedIdentifier(String),
}

#[derive(Debug, Clone, Default)]
pub struct ScopeContext {
    pub depth: usize,
    pub in_loop: bool,
    pub in_match: bool,
    pub in_macro: bool,
}

impl ScopeContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_block(&mut self) -> Self {
        let mut new = self.clone();
        new.depth += 1;
        new
    }

    pub fn enter_loop(&mut self) -> Self {
        let mut new = self.clone();
        new.in_loop = true;
        new.depth += 1;
        new
    }

    pub fn enter_match(&mut self) -> Self {
        let mut mut_self = self.clone();
        mut_self.in_match = true;
        mut_self.depth += 1;
        mut_self
    }

    pub fn enter_macro(&mut self) -> Self {
        let mut mut_self = self.clone();
        mut_self.in_macro = true;
        mut_self
    }
}

#[derive(Debug, Clone, Default)]
pub struct CafContext {
    scopes: Vec<HashMap<String, String>>,
    var_counter: usize,
    func_counter: usize,
    type_counter: usize,
    pub in_loop: bool,
    pub in_match: bool,
    pub in_macro: bool,
}

impl CafContext {
    pub fn new() -> Self {
        let mut scopes = Vec::new();
        scopes.push(HashMap::new());
        Self {
            scopes,
            var_counter: 0,
            func_counter: 0,
            type_counter: 0,
            in_loop: false,
            in_match: false,
            in_macro: false,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn declare_var(&mut self, name: &str) -> String {
        let scoped_name = format!("$V_{}", self.var_counter);
        self.var_counter += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scoped_name.clone());
        }
        scoped_name
    }

    pub fn declare_func(&mut self, name: &str) -> String {
        let scoped_name = format!("$F_{}", self.func_counter);
        self.func_counter += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scoped_name.clone());
        }
        scoped_name
    }

    pub fn declare_type(&mut self, name: &str) -> String {
        let scoped_name = format!("$T_{}", self.type_counter);
        self.type_counter += 1;
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), scoped_name.clone());
        }
        scoped_name
    }

    pub fn resolve(&self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(canonical) = scope.get(name) {
                return Some(canonical.clone());
            }
        }
        None
    }

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    pub fn scope_context(&self) -> ScopeContext {
        ScopeContext {
            depth: self.depth(),
            in_loop: self.in_loop,
            in_match: self.in_match,
            in_macro: self.in_macro,
        }
    }

    pub fn enter_loop(&mut self) {
        self.in_loop = true;
        self.push_scope();
    }

    pub fn enter_match(&mut self) {
        self.in_match = true;
        self.push_scope();
    }

    pub fn enter_macro(&mut self) {
        self.in_macro = true;
    }
}

pub trait AlgebraResolver: Send + Sync {
    fn resolve(&self, node_kind: &str, language: &str, context: &ScopeContext) -> Commutativity;
}

pub struct DefaultAlgebraResolver {
    table: CommutativityTable,
}

impl DefaultAlgebraResolver {
    pub fn new() -> Self {
        Self {
            table: CommutativityTable::new(),
        }
    }
}

impl Default for DefaultAlgebraResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl AlgebraResolver for DefaultAlgebraResolver {
    fn resolve(&self, node_kind: &str, language: &str, context: &ScopeContext) -> Commutativity {
        let base = match language {
            "rust" => self.table.get_rust(node_kind),
            "python" => self.table.get_python(node_kind),
            _ => Commutativity::OrderSensitive,
        };

        if context.in_macro && base.is_order_sensitive() {
            return Commutativity::OrderSensitive;
        }

        if context.in_loop && node_kind == "break_expression" {
            return Commutativity::OrderSensitive;
        }

        base
    }
}

pub struct CommutativityTable {
    rust: HashMap<&'static str, Commutativity>,
    python: HashMap<&'static str, Commutativity>,
}

impl CommutativityTable {
    pub fn new() -> Self {
        let mut rust = HashMap::new();
        let mut python = HashMap::new();

        // RUST: Top-level items (OrderInsensitive - OIZ)
        rust.insert("function_item", Commutativity::OrderInsensitive);
        rust.insert("struct_item", Commutativity::OrderInsensitive);
        rust.insert("enum_item", Commutativity::OrderInsensitive);
        rust.insert("trait_item", Commutativity::OrderInsensitive);
        rust.insert("impl_item", Commutativity::OrderInsensitive);
        rust.insert("mod_item", Commutativity::OrderInsensitive);
        rust.insert("const_item", Commutativity::OrderInsensitive);
        rust.insert("static_item", Commutativity::OrderInsensitive);
        rust.insert("use_declaration", Commutativity::OrderInsensitive);
        rust.insert("extern_crate_declaration", Commutativity::OrderInsensitive);

        // RUST: Struct/Enum/Trait internals (OrderInsensitive)
        rust.insert("field_declaration", Commutativity::OrderInsensitive);
        rust.insert("variant", Commutativity::OrderInsensitive);
        rust.insert("attribute_item", Commutativity::OrderInsensitive);

        // RUST: Function body internals (OrderSensitive - OSZ)
        rust.insert("block", Commutativity::OrderSensitive);
        rust.insert("expression_statement", Commutativity::OrderSensitive);
        rust.insert("let_declaration", Commutativity::OrderSensitive);
        rust.insert("assignment_expression", Commutativity::OrderSensitive);
        rust.insert("call_expression", Commutativity::OrderSensitive);
        rust.insert("method_call_expression", Commutativity::OrderSensitive);
        rust.insert("return_expression", Commutativity::OrderSensitive);
        rust.insert("if_expression", Commutativity::OrderSensitive);
        rust.insert("match_expression", Commutativity::OrderSensitive);
        rust.insert("loop_expression", Commutativity::OrderSensitive);
        rust.insert("for_expression", Commutativity::OrderSensitive);
        rust.insert("while_expression", Commutativity::OrderSensitive);
        rust.insert("break_expression", Commutativity::OrderSensitive);
        rust.insert("continue_expression", Commutativity::OrderSensitive);

        // RUST: Expressions with hybrid semantics (SHZ)
        rust.insert("binary_expression", Commutativity::Hybrid);
        rust.insert("unary_expression", Commutativity::Hybrid);
        rust.insert("closure_expression", Commutativity::Hybrid);

        // RUST: Types (OrderInsensitive)
        rust.insert("primitive_type", Commutativity::OrderInsensitive);
        rust.insert("generic_type", Commutativity::OrderInsensitive);
        rust.insert("tuple_type", Commutativity::OrderInsensitive);
        rust.insert("array_type", Commutativity::OrderInsensitive);
        rust.insert("reference_type", Commutativity::OrderInsensitive);
        rust.insert("pointer_type", Commutativity::OrderInsensitive);

        // RUST: Macro (OrderSensitive)
        rust.insert("macro_invocation", Commutativity::OrderSensitive);
        rust.insert("macro_definition", Commutativity::OrderInsensitive);

        // RUST: Parameters (OrderSensitive)
        rust.insert("parameters", Commutativity::OrderSensitive);
        rust.insert("parameter", Commutativity::OrderSensitive);
        rust.insert("self_parameter", Commutativity::OrderSensitive);

        // PYTHON: Top-level items (OrderInsensitive)
        python.insert("function_definition", Commutativity::OrderInsensitive);
        python.insert("async_function_definition", Commutativity::OrderInsensitive);
        python.insert("class_definition", Commutativity::OrderInsensitive);
        python.insert("decorated_definition", Commutativity::OrderInsensitive);
        python.insert("import_statement", Commutativity::OrderInsensitive);
        python.insert("import_from_statement", Commutativity::OrderInsensitive);
        python.insert("global_statement", Commutativity::OrderInsensitive);

        // PYTHON: Class internals (OrderInsensitive)
        python.insert("attribute", Commutativity::OrderInsensitive);

        // PYTHON: Function body (OrderSensitive)
        python.insert("block", Commutativity::OrderSensitive);
        python.insert("expression_statement", Commutativity::OrderSensitive);
        python.insert("assignment", Commutativity::OrderSensitive);
        python.insert("augmented_assignment", Commutativity::OrderSensitive);
        python.insert("return_statement", Commutativity::OrderSensitive);
        python.insert("raise_statement", Commutativity::OrderSensitive);
        python.insert("assert_statement", Commutativity::OrderSensitive);
        python.insert("if_statement", Commutativity::OrderSensitive);
        python.insert("for_statement", Commutativity::OrderSensitive);
        python.insert("while_statement", Commutativity::OrderSensitive);
        python.insert("try_statement", Commutativity::OrderSensitive);
        python.insert("with_statement", Commutativity::OrderSensitive);
        python.insert("match_statement", Commutativity::OrderSensitive);

        // PYTHON: Function calls (OrderSensitive)
        python.insert("call", Commutativity::OrderSensitive);

        // PYTHON: Parameters (OrderSensitive)
        python.insert("parameters", Commutativity::OrderSensitive);
        python.insert("parameter", Commutativity::OrderSensitive);
        python.insert("default_parameter", Commutativity::OrderSensitive);
        python.insert("positional_separator", Commutativity::OrderSensitive);
        python.insert("keyword_separator", Commutativity::OrderSensitive);

        // PYTHON: Decorators (OrderInsensitive)
        python.insert("decorator", Commutativity::OrderInsensitive);

        // PYTHON: Expressions (Hybrid)
        python.insert("binary_operator", Commutativity::Hybrid);
        python.insert("unary_operator", Commutativity::Hybrid);
        python.insert("lambda", Commutativity::Hybrid);

        Self { rust, python }
    }

    pub fn get_rust(&self, kind: &str) -> Commutativity {
        self.rust
            .get(kind)
            .copied()
            .unwrap_or(Commutativity::OrderSensitive)
    }

    pub fn get_python(&self, kind: &str) -> Commutativity {
        self.python
            .get(kind)
            .copied()
            .unwrap_or(Commutativity::OrderSensitive)
    }
}

impl Default for CommutativityTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticKind {
    pub fn from_tree_sitter(kind: &str) -> Self {
        match kind {
            "function_item" => SemanticKind::Function,
            "function_definition" => SemanticKind::Function,
            "method_declaration" | "method_definition" => SemanticKind::Method,
            "class_definition" => SemanticKind::Class,
            "struct_item" => SemanticKind::Struct,
            "enum_item" => SemanticKind::Enum,
            "trait_item" => SemanticKind::Trait,
            "impl_item" => SemanticKind::Impl,
            "mod_item" | "module" => SemanticKind::Module,
            "const_item" | "constant" => SemanticKind::Constant,
            "variable_declarator" | "assignment_target" => SemanticKind::Variable,
            "field_declaration" | "attribute" => SemanticKind::Field,
            "parameter" | "parameters" => SemanticKind::Parameter,
            "block" | "statement_block" => SemanticKind::Block,
            "expression_statement" | "statement" => SemanticKind::Statement,
            "identifier" => SemanticKind::Variable,
            "type_identifier" | "primitive_type" | "generic_type" => SemanticKind::Type,
            "import_statement" | "import_from_statement" => SemanticKind::Import,
            "use_statement" | "use_declaration" => SemanticKind::Use,
            "call_expression" | "call" => SemanticKind::Call,
            "macro_invocation" | "macro_definition" => SemanticKind::Macro,
            "attribute_item" | "decorator" => SemanticKind::Attribute,
            "match_expression" | "match_case" => SemanticKind::Match,
            "for_expression" | "while_expression" | "loop_expression" => SemanticKind::Loop,
            _ => SemanticKind::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_function_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(
            table.get_rust("function_item"),
            Commutativity::OrderInsensitive
        );
    }

    #[test]
    fn test_rust_struct_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(
            table.get_rust("struct_item"),
            Commutativity::OrderInsensitive
        );
    }

    #[test]
    fn test_rust_block_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(table.get_rust("block"), Commutativity::OrderSensitive);
    }

    #[test]
    fn test_python_function_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(
            table.get_python("function_definition"),
            Commutativity::OrderInsensitive
        );
    }

    #[test]
    fn test_python_class_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(
            table.get_python("class_definition"),
            Commutativity::OrderInsensitive
        );
    }

    #[test]
    fn test_python_call_commutativity() {
        let table = CommutativityTable::new();
        assert_eq!(table.get_python("call"), Commutativity::OrderSensitive);
    }

    #[test]
    fn test_semantic_kind_from_tree_sitter() {
        assert_eq!(
            SemanticKind::from_tree_sitter("function_item"),
            SemanticKind::Function
        );
        assert_eq!(
            SemanticKind::from_tree_sitter("struct_item"),
            SemanticKind::Struct
        );
        assert_eq!(
            SemanticKind::from_tree_sitter("call_expression"),
            SemanticKind::Call
        );
    }

    #[test]
    fn test_commutativity_table_is_single_source() {
        let table = CommutativityTable::new();
        let rust_fn = table.get_rust("function_item");
        assert_eq!(rust_fn, Commutativity::OrderInsensitive);

        let rust_struct = table.get_rust("struct_item");
        assert_eq!(rust_struct, Commutativity::OrderInsensitive);
    }

    #[test]
    fn test_caf_context_var_declaration() {
        let mut ctx = CafContext::new();
        let canonical = ctx.declare_var("my_var");
        assert_eq!(canonical, "$V_0");
        assert_eq!(ctx.resolve("my_var"), Some("$V_0".to_string()));
    }

    #[test]
    fn test_caf_context_scope_isolation() {
        let mut ctx = CafContext::new();
        ctx.declare_var("x");
        ctx.push_scope();
        ctx.declare_var("x");

        assert_eq!(ctx.resolve("x"), Some("$V_1".to_string()));

        ctx.pop_scope();
        assert_eq!(ctx.resolve("x"), Some("$V_0".to_string()));
    }

    #[test]
    fn test_caf_context_func_and_type() {
        let mut ctx = CafContext::new();
        let func = ctx.declare_func("process_data");
        let type_name = ctx.declare_type("User");

        assert_eq!(func, "$F_0");
        assert_eq!(type_name, "$T_0");
    }

    #[test]
    fn test_scope_context_enter_block() {
        let mut ctx = ScopeContext::new();
        let deeper = ctx.enter_block();

        assert_eq!(deeper.depth, 1);
    }

    #[test]
    fn test_scope_context_loop() {
        let mut ctx = ScopeContext::new();
        let loop_ctx = ctx.enter_loop();

        assert!(loop_ctx.in_loop);
    }
}
