use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct EdgeEvent {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub language: String,
    pub source_file: String,
    pub line: usize,
    pub confidence: f32,
    pub raw_text: String,
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
}

fn default_schema_version() -> u32 {
    1
}
