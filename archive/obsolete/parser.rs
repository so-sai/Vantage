use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tree_sitter::{Language, Node, Parser};
use vantage_types::{CognitiveSignal, Origin, SymbolKind};

pub struct EpistemicParser {
    parser: Parser,
    language_name: String,
    solid_kinds: Vec<String>,
}

impl EpistemicParser {
    pub fn new(lang: Language, language_name: &str, solid_kinds: Vec<&str>) -> Result<Self, String> {
        let mut parser = Parser::new();
        parser
            .set_language(&lang)
            .map_err(|_| "Failed to load language")?;

        Ok(Self {
            parser,
            language_name: language_name.to_string(),
            solid_kinds: solid_kinds.iter().map(|s| s.to_string()).collect(),
        })
    }

    pub fn new_rust_parser() -> Result<Self, String> {
        Self::new(
            tree_sitter_rust::LANGUAGE.into(),
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
            ],
        )
    }

    pub fn new_python_parser() -> Result<Self, String> {
        Self::new(
            tree_sitter_python::LANGUAGE.into(),
            "python",
            vec![
                "function_definition",
                "class_definition",
                "decorated_definition",
                "async_function_definition",
            ],
        )
    }

    pub fn parse_signals(&mut self, source: &str) -> Vec<CognitiveSignal> {
        // Normalization: Replace ALL non-standard Unicode whitespace and control characters
        // with byte-equivalent ASCII spaces. This prevents tree-sitter from treating 
        // radioactive bytes (like \0 or NBSP) as semantic markers that break the AST,
        // while strictly preserving byte offsets for stable signaling.
        let mut normalized = String::with_capacity(source.len());
        for c in source.chars() {
            // Standard whitespace/control we allow: space, tab, newline, carriage return
            if (c.is_whitespace() || c.is_control()) && c != ' ' && c != '\t' && c != '\n' && c != '\r' {
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
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");

        // Phase A: Extraction (Finding the Anchor)
        if let Some(uuid) = Self::extract_uuid(text) {
            // Phase B: Normalization (Geometric Target -> Signal)
            if let Some(target_node) = self.find_geometric_target(node, source) {
                let signal = self.map_to_signal(uuid, target_node, source);
                signals.push(signal);
            }
        }

        for child in node.children(&mut node.walk()) {
            self.extract_and_normalize(child, source, signals);
        }
    }

    fn find_geometric_target<'a>(&self, anchor: Node<'a>, _source: &str) -> Option<Node<'a>> {
        let mut current_node = anchor;

        while let Some(sibling) = current_node.next_sibling() {
            if let Some(target) = self.search_for_solid_target(sibling) {
                return Some(target);
            }
            // If the sibling was a comment or something we decided to skip, continue the loop
            current_node = sibling;
        }
        None
    }

    fn search_for_solid_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let kind = node.kind();

        // 1. If it's a solid kind we recognize, return it
        if node.is_named() && self.solid_kinds.iter().any(|k| k == kind) {
            return Some(node);
        }

        // 2. If it's a comment, we want to skip it (return None so the caller continues to next sibling)
        if kind.contains("comment") || kind == "html_block" {
            return None;
        }

        // 3. For any other node (like ERROR or anonymous junk), search its children
        for child in node.children(&mut node.walk()) {
            if let Some(target) = self.search_for_solid_target(child) {
                return Some(target);
            }
        }

        None
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
        
        // Semantic Hash (placeholder: stripping whitespace for now)
        let semantic_content: Vec<u8> = content.iter().filter(|&&b| !b.is_ascii_whitespace()).cloned().collect();
        let semantic_hash = Self::compute_hash(&semantic_content);

        CognitiveSignal {
            uuid,
            symbol_id,
            parent: None, // TODO: Implement hierarchy traversal
            symbol_kind,
            language: self.language_name.clone(),
            structural_hash,
            semantic_hash,
            signature: None, // TODO: Implement signature extraction
            metadata: HashMap::new(),
            origin: Origin {
                parser: format!("tree-sitter-{}", self.language_name),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            confidence: 1.0, // Exact AST match
        }
    }

    fn extract_symbol_id(&self, node: Node, source: &str) -> String {
        // Find identifier child
        if let Some(id_node) = node.child_by_field_name("name") {
            return id_node.utf8_text(source.as_bytes()).unwrap_or("unknown").to_string();
        }
        // Fallback for nodes without named identifiers in tree-sitter grammars
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
