use crate::symbol_id::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IdentityAnchor {
    /// For nodes that define or use a symbol (variables, functions, types)
    Binding(SymbolId),

    /// For anonymous operators (e.g., +, -, *)
    /// Contains (operator_kind, arity)
    Operator(String, usize),

    /// For literal values (strings, numbers)
    /// Contains a canonical hash of the content
    Literal(String),

    /// For structural nodes without specific identity (e.g., blocks, if-statements)
    /// Differentiates nodes based on their semantic role within the parent
    Structural,
}

impl IdentityAnchor {
    pub fn to_stable_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            IdentityAnchor::Binding(sym) => {
                buf.push(0x01);
                let name = crate::symbol_id::registry().get_name(sym);
                let fqn_bytes = name.as_bytes();
                buf.extend((fqn_bytes.len() as u32).to_le_bytes());
                buf.extend(fqn_bytes);
            }
            IdentityAnchor::Operator(kind, arity) => {
                buf.push(0x02);
                let k_bytes = kind.as_bytes();
                assert!(
                    k_bytes.len() <= (u32::MAX as usize),
                    "Operator kind name too long"
                );
                buf.extend((k_bytes.len() as u32).to_le_bytes());
                buf.extend(k_bytes);
                buf.extend((*arity as u64).to_le_bytes());
            }
            IdentityAnchor::Literal(hash) => {
                buf.push(0x03);
                let h_bytes = hash.as_bytes();
                buf.extend((h_bytes.len() as u32).to_le_bytes());
                buf.extend(h_bytes);
            }
            IdentityAnchor::Structural => {
                buf.push(0x04);
            }
        }
        buf
    }
}
