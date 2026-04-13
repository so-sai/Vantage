use crate::semantic_role::SemanticRole;

pub struct RoleResolver;

impl RoleResolver {
    pub fn resolve(kind: &str, parent_kind: &str, _pos: usize) -> SemanticRole {
        match kind {
            // Function level
            "function_item" | "function_definition" => SemanticRole::FunctionBody,
            "parameters" | "formal_parameters" => SemanticRole::ParameterList,
            "parameter" | "parameter_declaration" | "identifier" if parent_kind == "parameters" => {
                SemanticRole::Parameter
            }

            // Bindings
            "let_declaration" | "variable_declaration" => SemanticRole::LetBinding,
            "variable_declarator" | "pattern" if parent_kind.contains("declaration") => {
                SemanticRole::SymbolDeclaration
            }
            "init" | "value" | "expression" if parent_kind.contains("declarator") => {
                SemanticRole::Initializer
            }

            // Control Flow
            "block" | "statement_block" => SemanticRole::Block,
            "if_let_expression" | "match_arm" => SemanticRole::GuardScope,

            // Collections
            "struct_item" | "class_definition" => SemanticRole::ModuleItem,
            "field_declaration" | "attribute" => SemanticRole::StructField,

            // Fallback to kind-based heuristics if parent isn't enough
            _ => Self::heuristic_resolve(kind),
        }
    }

    fn heuristic_resolve(kind: &str) -> SemanticRole {
        if kind.contains("expression") {
            SemanticRole::Expression
        } else if kind.contains("statement") {
            SemanticRole::Statement
        } else if kind.contains("identifier") {
            SemanticRole::SymbolDeclaration
        } else if kind.contains("literal") {
            SemanticRole::Literal
        } else {
            SemanticRole::SyntaxNode
        }
    }
}
