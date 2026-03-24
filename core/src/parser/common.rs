use vantage_types::signal::StructuralSignal;

pub trait EpistemicParser {
    /// Parses the given content and returns a list of structural signals.
    fn parse_signals(&mut self, content: &str, file_path: &str) -> Vec<StructuralSignal>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    Toml,
    Json,
    Markdown,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "toml" => Some(Language::Toml),
            "json" => Some(Language::Json),
            "md" => Some(Language::Markdown),
            _ => None,
        }
    }
}
