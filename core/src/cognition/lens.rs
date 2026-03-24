use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BlockType {
    Heading { level: u32 },
    Paragraph,
    Code { lang: Option<String> },
    List { ordered: bool },
    Table,
    BlockQuote,
    ThematicBreak,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: String,
    pub kind: BlockType,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensData {
    pub blocks: Vec<Block>,
}

// NOTE: Drift detection, Remediation, and Sealing logic removed in v1.2.3.
// These responsibilities are now delegated to the 'kit' host.
