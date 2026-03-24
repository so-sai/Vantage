use tree_sitter::{Node, Parser};
use vantage_types::signal::{StructuralSignal, Origin};
use vantage_types::symbol::SymbolKind;
use crate::fingerprint::hasher::Hasher;
use crate::parser::common::EpistemicParser;
use std::collections::HashMap;

pub struct RustParser {
    parser: Parser,
    solid_kinds: Vec<&'static str>,
}

impl RustParser {
    pub fn new() -> Self {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .expect("Failed to load Rust language");

        Self {
            parser,
            solid_kinds: vec![
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
        }
    }

    fn extract_uuid(text: &str) -> Option<String> {
        let marker = "@epistemic:";
        if let Some(pos) = text.find(marker) {
            let uuid_part = &text[pos + marker.len()..];
            let raw_uuid = uuid_part.split_whitespace().next()?;
            let clean_uuid = raw_uuid.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
            return Some(clean_uuid.to_string());
        }
        None
    }

    fn find_geometric_target<'a>(&self, anchor: Node<'a>) -> Option<Node<'a>> {
        let mut current_node = anchor;
        while let Some(sibling) = current_node.next_sibling() {
            if let Some(target) = self.search_for_solid_target(sibling) {
                return Some(target);
            }
            current_node = sibling;
        }
        None
    }

    fn search_for_solid_target<'a>(&self, node: Node<'a>) -> Option<Node<'a>> {
        let kind = node.kind();
        if node.is_named() && self.solid_kinds.iter().any(|&k| k == kind) {
            return Some(node);
        }
        if kind.contains("comment") || kind == "html_block" {
            return None;
        }
        for child in node.children(&mut node.walk()) {
            if let Some(target) = self.search_for_solid_target(child) {
                return Some(target);
            }
        }
        None
    }

    fn map_to_signal(&self, uuid: String, node: Node, source: &str, _file_path: &str) -> StructuralSignal {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        let kind_str = node.kind();
        
        let symbol_kind = match kind_str {
            "function_item" => SymbolKind::Function,
            "struct_item" => SymbolKind::Struct,
            "trait_item" => SymbolKind::Trait,
            _ => SymbolKind::Other(kind_str.to_string()),
        };

        let symbol_id = node.child_by_field_name("name")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .unwrap_or("unnamed")
            .to_string();

        let structural_hash = Hasher::structural_hash(text);
        let normalized_hash = Hasher::normalized_hash(text);

        StructuralSignal {
            uuid,
            symbol_id,
            parent: None,
            symbol_kind,
            language: "rust".to_string(),
            structural_hash,
            normalized_hash,
            signature: text.lines().next().map(|s| s.to_string()),
            metadata: HashMap::new(),
            origin: Origin {
                parser: "tree-sitter-rust".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            confidence: 1.0,
        }
    }

    fn extract_all(&self, node: Node, source: &str, file_path: &str, signals: &mut Vec<StructuralSignal>) {
        let text = node.utf8_text(source.as_bytes()).unwrap_or("");
        if let Some(uuid) = Self::extract_uuid(text) {
            if let Some(target_node) = self.find_geometric_target(node) {
                let signal = self.map_to_signal(uuid, target_node, source, file_path);
                signals.push(signal);
            }
        }
        for child in node.children(&mut node.walk()) {
            self.extract_all(child, source, file_path, signals);
        }
    }
}

impl EpistemicParser for RustParser {
    fn parse_signals(&mut self, content: &str, file_path: &str) -> Vec<StructuralSignal> {
        let tree = self.parser.parse(content, None).expect("Parse failed");
        let root = tree.root_node();
        let mut signals = Vec::new();
        self.extract_all(root, content, file_path, &mut signals);
        signals
    }
}
