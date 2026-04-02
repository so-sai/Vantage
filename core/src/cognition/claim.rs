use super::label::CognitiveLabel;
use serde::{Deserialize, Serialize};
use vantage_types::CognitiveSignal;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClaimType {
    /// Code uses f-strings or template literals with variables
    TemplateInterpolation,
    /// Code contains string literals
    StringLiteral,
    /// Function definition detected
    FunctionDefinition,
    /// Struct/class definition detected
    TypeDefinition,
    /// Code uses macro invocations
    MacroUsage,
    /// Code contains unsafe blocks (Rust)
    UnsafeUsage,
    /// Generic structural claim
    Structural(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Seal {
    pub sealed_by: String,
    pub sealed_at_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveClaim {
    pub id: String,
    pub signal_uuid: String,
    pub claim_type: ClaimType,
    pub label: CognitiveLabel,
    pub confidence: f32,
    pub source_file: String,
    pub sealed: Option<Seal>,
}

impl CognitiveClaim {
    pub fn from_signal(signal: &CognitiveSignal, source_file: &str) -> Vec<Self> {
        let mut claims = Vec::new();

        let claim_types = classify_signal(signal);

        for (claim_type, confidence) in claim_types {
            claims.push(CognitiveClaim {
                id: format!("{}::{:?}", signal.uuid, claim_type),
                signal_uuid: signal.uuid.clone(),
                claim_type,
                label: CognitiveLabel::Fact,
                confidence,
                source_file: source_file.to_string(),
                sealed: None,
            });
        }

        claims
    }
}

fn classify_signal(signal: &CognitiveSignal) -> Vec<(ClaimType, f32)> {
    let mut claims = Vec::new();

    match &signal.symbol_kind {
        vantage_types::SymbolKind::Function => {
            claims.push((ClaimType::FunctionDefinition, 1.0));
        }
        vantage_types::SymbolKind::Struct | vantage_types::SymbolKind::Class => {
            claims.push((ClaimType::TypeDefinition, 1.0));
        }
        _ => {}
    }

    if let Some(sig) = &signal.signature
        && (sig.contains("f\"") || sig.contains("f'") || sig.contains("format!"))
    {
        claims.push((ClaimType::TemplateInterpolation, 0.9));
    }

    claims.push((
        ClaimType::Structural(format!("{:?}", signal.symbol_kind)),
        0.5,
    ));

    claims
}
