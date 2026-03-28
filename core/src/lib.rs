// ============================================================
// VANTAGE CORE v1.2.3 - Structural Sensor Engine
// ============================================================
pub mod cognition;
pub mod fingerprint;
pub mod intent;
pub mod parser;

pub use vantage_types::signal::{CognitiveSignal, Origin};
pub use vantage_types::symbol::SymbolKind;
pub use cognition::lens::{LensData, Block, BlockType};
pub use cognition::enforcer::{enforce_claim, EnforcementDecision, ExecutionContext};
pub use parser::{get_parser, EpistemicParser, Language};
pub use fingerprint::hasher::Hasher;

use std::fs;
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VantageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("Normalization failed: {0}")]
    NormalizationError(String),
}

#[derive(Debug, serde::Serialize)]
pub struct Metadata {
    pub word_count: usize,
    pub last_modified: Option<SystemTime>,
    pub source_type: String,
}

#[derive(Debug, serde::Serialize)]
pub struct DocumentData {
    pub text: String,
    pub segments: Vec<Block>,
    pub signals: Vec<CognitiveSignal>,
    pub metadata: Metadata,
}

/// The core entry point for document sensing and structural abstraction.
pub fn document_lens(path: &str) -> Result<DocumentData, VantageError> {
    let path_obj = Path::new(path);
    let extension = path_obj
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| VantageError::UnsupportedFileType("No extension".to_string()))?;

    let mut text = String::new();
    let mut segments = Vec::new();
    let mut signals = Vec::new();

    let last_modified = fs::metadata(path_obj).ok().and_then(|m| m.modified().ok());

    match extension {
        "rs" | "py" => {
            let content = fs::read_to_string(path_obj)?;
            text = content.clone();
            
            let lang = Language::from_extension(extension).ok_or_else(|| {
                VantageError::UnsupportedFileType(extension.to_string())
            })?;
            
            let mut parser = get_parser(lang);
            signals = parser.parse_signals(&content, path);
            
            for signal in &signals {
                segments.push(Block {
                    id: signal.uuid.clone(),
                    kind: BlockType::Code { lang: Some(signal.language.clone()) },
                    content: signal.signature.clone().unwrap_or_default(),
                    start_line: 0, // Placeholder
                    end_line: 0, // Placeholder
                    metadata: std::collections::HashMap::new(),
                    hash: signal.structural_hash.clone(),
                });
            }
        }
        _ => return Err(VantageError::UnsupportedFileType(extension.to_string())),
    }

    let word_count = text.split_whitespace().count();
    Ok(DocumentData {
        text,
        segments,
        signals,
        metadata: Metadata {
            word_count,
            last_modified,
            source_type: extension.to_string(),
        },
    })
}
