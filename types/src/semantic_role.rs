use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[repr(u16)]
pub enum SemanticRole {
    Root = 0,

    // Functions
    FunctionName = 10,
    ParameterList = 11,
    Parameter = 12,
    ReturnType = 13,
    FunctionBody = 14,

    // Blocks & Control Flow
    Block = 20,
    Statement = 21,
    GuardScope = 22,
    MatchArm = 23,

    // Bindings
    LetBinding = 30,
    Initializer = 31,
    SymbolDeclaration = 32,

    // Expressions
    Expression = 40,
    CallTarget = 41,
    CallArgument = 42,
    Literal = 43,
    Operator = 44,

    // Types
    TypeReference = 50,
    TypeParameter = 51,

    // Collections
    StructField = 60,
    EnumVariant = 61,
    ClassMember = 62,

    // Modules & Imports
    ImportItem = 70,
    ModuleItem = 71,

    // Fallback
    #[default]
    SyntaxNode = 999,
}
