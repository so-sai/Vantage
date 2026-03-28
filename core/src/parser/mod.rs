use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};
use vantage_types::{CognitiveSignal, Origin, SymbolKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            _ => None,
        }
    }
}

pub struct EpistemicParser {
    parser: Parser,
    language_name: String,
    solid_kinds: Vec<String>,
}

impl EpistemicParser {
    pub fn new(lang_name: &str, solid_kinds: Vec<&str>) -> Result<Self, String> {
        let mut parser = Parser::new();
        let lang = match lang_name {
            "rust" => tree_sitter_rust::LANGUAGE.into(),
            "python" => tree_sitter_python::LANGUAGE.into(),
            _ => return Err(format!("Unsupported language: {}", lang_name)),
        };
        
        parser
            .set_language(&lang)
            .map_err(|_| "Failed to load language")?;

        Ok(Self {
            parser,
            language_name: lang_name.to_string(),
            solid_kinds: solid_kinds.iter().map(|s| s.to_string()).collect(),
        })
    }

    pub fn new_rust_parser() -> Result<Self, String> {
        Self::new(
            "rust",
            vec![
                "function_item",
                "struct_item",
                "impl_item",
                "enum_item",
                "trait_item",
                "const_item",
                "static_item",
                "mod_item",
                "macro_invocation",
                "expression_statement",
                "call_expression",
            ],
        )
    }

    pub fn new_python_parser() -> Result<Self, String> {
        Self::new(
            "python",
            vec![
                "function_definition",
                "class_definition",
                "decorated_definition",
                "async_function_definition",
            ],
        )
    }

    pub fn parse_signals(&mut self, source: &str, _path: &str) -> Vec<CognitiveSignal> {
        // Normalization: Replace ALL non-standard Unicode whitespace and control characters
        // with byte-equivalent ASCII spaces. This prevents tree-sitter from treating 
        // radioactive bytes (like \0 or NBSP) as semantic markers that break the AST,
        // while strictly preserving byte offsets for stable signaling.
        let mut normalized = String::with_capacity(source.len());
        for c in source.chars() {
            // Standard printable ASCII and standard control/whitespace we allow
            let is_safe = (c >= '\u{0020}' && c <= '\u{007E}') || 
                         c == '\n' || c == '\r' || c == '\t';
            
            // Non-standard line breakers that must force a newline to terminate comments
            let is_line_breaker = c == '\u{2028}' || c == '\u{2029}' || c == '\u{000b}' || c == '\u{000c}';

            // Check for Private Use Area
            let is_private = (c >= '\u{E000}' && c <= '\u{F8FF}') || 
                             (c >= '\u{F0000}' && c <= '\u{FFFFF}') || 
                             (c >= '\u{100000}' && c <= '\u{10FFFF}');

            if is_line_breaker {
                // Must force a line break to stop comment swallowing
                normalized.push('\n');
                for _ in 1..c.len_utf8() {
                    normalized.push(' ');
                }
            } else if (!is_safe || is_private || c.is_control()) && c != '\n' && c != '\r' && c != '\t' {
                // Replace with N spaces where N is the byte-length of the original character
                for _ in 0..c.len_utf8() {
                    normalized.push(' ');
                }
            } else {
                normalized.push(c);
            }
        }
        
        // Vantage v1.2.3: Zero-Corruption Normalization Layer
        let tree = self.parser.parse(&normalized, None).expect("Parse failed");
        let root = tree.root_node();
        let mut signals = Vec::new();

        self.extract_and_normalize(root, &normalized, &mut signals);

        signals
    }

    /// Phase A & B integrated traversal
    fn extract_and_normalize(&self, node: Node, source: &str, signals: &mut Vec<CognitiveSignal>) {
        let kind = node.kind();
        
        // Phase A: Extraction (Finding the Anchor)
        // We ONLY look for anchors in comment nodes to avoid root-level swallowing or collision.
        if kind.contains("comment") || kind == "html_block" {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            if let Some(uuid) = Self::extract_uuid(text) {
                // Phase B: Normalization (Geometric Target -> Signal)
                if let Some(target_node) = self.find_geometric_target(node, source) {
                    let signal = self.map_to_signal(uuid, target_node, source);
                    signals.push(signal);
                }
            }
        }

        for child in node.children(&mut node.walk()) {
            self.extract_and_normalize(child, source, signals);
        }
    }

    pub fn find_geometric_target<'a>(&self, anchor: Node<'a>, _source: &str) -> Option<Node<'a>> {
        let anchor_end = anchor.end_byte();
        
        let mut root = anchor;
        while let Some(parent) = root.parent() {
            root = parent;
        }

        self.find_first_solid_after_byte(root, anchor_end)
    }

    fn find_first_solid_after_byte<'a>(&self, root: Node<'a>, offset: usize) -> Option<Node<'a>> {
        let mut cursor = root.walk();
        
        // We need to visit EVERY node in pre-order to find the first solid one that starts after offset.
        // We use cursor for efficiency.
        loop {
            let node = cursor.node();
            
            if node.start_byte() >= offset {
                if let Some(target) = self.as_solid_target(node) {
                    return Some(target);
                }
            }

            if cursor.goto_first_child() {
                continue;
            }

            loop {
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return None;
                }
            }
        }
    }

    fn as_solid_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let kind = node.kind();
        if node.is_named() && self.solid_kinds.iter().any(|k| k == kind) {
            return Some(node);
        }
        None
    }

    fn search_for_solid_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        self.as_solid_target(node)
    }

    fn map_to_signal(&self, uuid: String, node: Node, source: &str) -> CognitiveSignal {
        let span = node.byte_range();
        let content = &source.as_bytes()[span.clone()];
        let kind_str = node.kind();

        // Normalized Mapping
        let symbol_kind = match self.language_name.as_str() {
            "rust" => match kind_str {
                "function_item" => SymbolKind::Function,
                "struct_item" => SymbolKind::Struct,
                "enum_item" => SymbolKind::Enum,
                "trait_item" => SymbolKind::Trait,
                "mod_item" => SymbolKind::Other("module".to_string()),
                _ => SymbolKind::Other(kind_str.to_string()),
            },
            "python" => match kind_str {
                "function_definition" | "async_function_definition" => SymbolKind::Function,
                "class_definition" => SymbolKind::Class,
                _ => SymbolKind::Other(kind_str.to_string()),
            },
            _ => SymbolKind::Other(kind_str.to_string()),
        };

        let symbol_id = self.extract_symbol_id(node, source);
        let structural_hash = Self::compute_hash(content);
        
        // Semantic Hash
        let semantic_content: Vec<u8> = content.iter().filter(|&&b| !b.is_ascii_whitespace()).cloned().collect();
        let semantic_hash = Self::compute_hash(&semantic_content);

        CognitiveSignal {
            uuid,
            symbol_id,
            parent: None,
            symbol_kind,
            language: self.language_name.clone(),
            structural_hash,
            semantic_hash,
            signature: None,
            metadata: HashMap::new(),
            origin: Origin {
                parser: format!("tree-sitter-{}", self.language_name),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            confidence: 1.0,
        }
    }

    fn extract_symbol_id(&self, node: Node, source: &str) -> String {
        if let Some(id_node) = node.child_by_field_name("name") {
            return id_node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
        }
        if let Some(id_node) = node.child_by_field_name("function") {
            return id_node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
        }
        
        // Fallback for ERROR nodes: find the first identifier
        if node.kind() == "ERROR" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return child.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
                }
            }
        }

        "unnamed".to_string()
    }

    fn extract_uuid(_text: &str) -> Option<String> {
        let marker = "@epistemic:";
        if let Some(pos) = _text.find(marker) {
            let uuid_part = &_text[pos + marker.len()..];
            let raw_uuid = uuid_part.split_whitespace().next()?;
            let clean_uuid = raw_uuid.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
            return Some(clean_uuid.to_string());
        }
        None
    }

    fn compute_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

pub fn get_parser(lang: Language) -> EpistemicParser {
    match lang {
        Language::Rust => EpistemicParser::new_rust_parser().unwrap(),
        Language::Python => EpistemicParser::new_python_parser().unwrap(),
    }
}
