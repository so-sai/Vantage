pub mod common;
pub mod rust;
pub mod python;

pub use common::{EpistemicParser, Language};

pub fn get_parser(lang: Language) -> Box<dyn EpistemicParser> {
    match lang {
        Language::Rust => Box::new(rust::RustParser::new()),
        Language::Python => Box::new(python::PythonParser::new()),
        Language::Toml | Language::Json | Language::Markdown => {
            // Placeholders for v1.2.3
            // In RC1, we prioritize Code Sensor stability.
            // Config Sensor logic will follow in v1.2.4.
            Box::new(rust::RustParser::new()) // Fallback to avoid panic in verify tool
        }
    }
}
