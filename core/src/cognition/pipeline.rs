use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::cognition::claim::CognitiveClaim;
use crate::cognition::decision::{Decision, InvariantEngine, Verdict};
use crate::parser::{EpistemicParser, Language};
use vantage_types::CognitiveSignal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub file: String,
    pub signals: Vec<CognitiveSignal>,
    pub claims: Vec<CognitiveClaim>,
    pub verdicts: Vec<Verdict>,
    pub final_decision: Decision,
    /// Pipeline execution time in milliseconds. Kit monitors this for regression.
    pub duration_ms: u64,
}

pub struct Pipeline {
    pub parser: EpistemicParser,
    pub engine: InvariantEngine,
}

impl Pipeline {
    pub fn new(lang: Language) -> Result<Self, String> {
        let parser = match lang {
            Language::Rust => EpistemicParser::new_rust_parser()?,
            Language::Python => EpistemicParser::new_python_parser()?,
            Language::Ruby => EpistemicParser::new_ruby_parser()?,
            Language::Javascript => EpistemicParser::new_javascript_parser()?,
            Language::Typescript => EpistemicParser::new_typescript_parser()?,
            Language::Tsx => EpistemicParser::new_tsx_parser()?,
        };
        Ok(Self {
            parser,
            engine: InvariantEngine::new(),
        })
    }

    pub fn run(&mut self, source: &str, file_path: &str) -> PipelineResult {
        let start = Instant::now();

        let signals = self.parser.parse_signals(source, file_path);

        let claims: Vec<CognitiveClaim> = signals
            .iter()
            .flat_map(|s| CognitiveClaim::from_signal(s, file_path))
            .collect();

        let verdicts = self.engine.evaluate_all(&claims);

        let final_decision = if verdicts.iter().any(|v| v.decision == Decision::Reject) {
            Decision::Reject
        } else if verdicts.iter().any(|v| v.decision == Decision::Warn) {
            Decision::Warn
        } else {
            Decision::Allow
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        PipelineResult {
            file: file_path.to_string(),
            signals,
            claims,
            verdicts,
            final_decision,
            duration_ms,
        }
    }
}
