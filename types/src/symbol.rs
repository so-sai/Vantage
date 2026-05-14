use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Module,
    Constant,
    Variable,
    Interface,
    /// React/JSX component, Vue component, etc.
    Component,
    Other(String),
}
