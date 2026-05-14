pub mod adapter;

use vantage_types::uir::UnifiedGraph;
use crate::parser::{EpistemicParser, Language as ParserLang};

pub struct Normalizer;

impl Normalizer {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, source: &str, path: &str) -> Result<UnifiedGraph, String> {
        let ext = path
            .rsplit('.')
            .next()
            .unwrap_or("")
            .to_lowercase();

        let parser_lang = ParserLang::from_extension(&ext)
            .ok_or_else(|| format!("Unsupported file extension: .{}", ext))?;

        let parser_fn: fn() -> Result<EpistemicParser, String> = match parser_lang {
            ParserLang::Rust => EpistemicParser::new_rust_parser,
            ParserLang::Python => EpistemicParser::new_python_parser,
            ParserLang::Ruby => EpistemicParser::new_ruby_parser,
            ParserLang::Javascript => EpistemicParser::new_javascript_parser,
            ParserLang::Typescript => EpistemicParser::new_typescript_parser,
            ParserLang::Tsx => EpistemicParser::new_tsx_parser,
        };

        let mut parser = parser_fn()?;
        let (signals, graph) = parser.parse_all(source, path);

        let intent_blocks = crate::parser::intent_extractor::extract_intents(source);
        Ok(adapter::normalize_to_unified(&signals, &graph, &parser_lang, &intent_blocks))
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}
