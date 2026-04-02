use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};
use vantage_types::{CognitiveSignal, Origin, SourceLocation, SymbolKind};

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

    #[tracing::instrument(skip(self, source))]
    pub fn parse_signals(&mut self, source: &str, path: &str) -> Vec<CognitiveSignal> {
        // Normalization: Replace ALL non-standard Unicode whitespace and control characters
        // with byte-equivalent ASCII spaces. This prevents tree-sitter from treating
        // radioactive bytes (like \0 or NBSP) as semantic markers that break the AST,
        // while strictly preserving byte offsets for stable signaling.
        let mut normalized = String::with_capacity(source.len());
        for c in source.chars() {
            // Standard printable ASCII and standard control/whitespace we allow
            let is_safe =
                ('\u{0020}'..='\u{007E}').contains(&c) || c == '\n' || c == '\r' || c == '\t';

            // Non-standard line breakers that must force a newline to terminate comments
            let is_line_breaker =
                c == '\u{2028}' || c == '\u{2029}' || c == '\u{000b}' || c == '\u{000c}';

            // Check for Private Use Area
            let is_private = ('\u{E000}'..='\u{F8FF}').contains(&c)
                || ('\u{F0000}'..='\u{FFFFF}').contains(&c)
                || ('\u{100000}'..='\u{10FFFF}').contains(&c);

            if is_line_breaker {
                // Must force a line break to stop comment swallowing
                normalized.push('\n');
                for _ in 1..c.len_utf8() {
                    normalized.push(' ');
                }
            } else if (!is_safe || is_private || c.is_control())
                && c != '\n'
                && c != '\r'
                && c != '\t'
            {
                // Replace with N spaces where N is the byte-length of the original character
                for _ in 0..c.len_utf8() {
                    normalized.push(' ');
                }
            } else {
                normalized.push(c);
            }
        }

        // Vantage v1.2.3: Zero-Corruption Normalization Layer
        let tree = self.parser.parse(&normalized, None).unwrap_or_else(|| {
            tracing::error!("Structural parse failed for file: {}", path);
            std::process::exit(1); // Still prefer exit over panic in binary
        });
        let root = tree.root_node();
        let mut signals = Vec::new();

        self.extract_and_normalize(root, &normalized, &mut signals, path);

        // Determinism: sort signals by byte_start for stable cross-platform ordering.
        signals.sort_by_key(|s| s.location.byte_start);

        signals
    }

    /// Parse source and return both signals and dependency graph.
    #[tracing::instrument(skip(self, source))]
    pub fn parse_with_graph(
        &mut self,
        source: &str,
        path: &str,
    ) -> (Vec<CognitiveSignal>, crate::graph::SymbolGraph) {
        use crate::graph::SymbolGraph;

        // Normalize same as parse_signals
        let mut normalized = String::with_capacity(source.len());
        for c in source.chars() {
            let is_safe =
                ('\u{0020}'..='\u{007E}').contains(&c) || c == '\n' || c == '\r' || c == '\t';
            let is_line_breaker =
                c == '\u{2028}' || c == '\u{2029}' || c == '\u{000b}' || c == '\u{000c}';
            let is_private = ('\u{E000}'..='\u{F8FF}').contains(&c)
                || ('\u{F0000}'..='\u{FFFFF}').contains(&c)
                || ('\u{100000}'..='\u{10FFFF}').contains(&c);
            let is_control = c.is_control() && c != '\n' && c != '\r' && c != '\t';

            if is_safe {
                normalized.push(c);
            } else if is_line_breaker {
                normalized.push('\n');
            } else if is_private || is_control {
                normalized.push(' ');
            } else {
                normalized.push(c);
            }
        }

        let tree = self.parser.parse(&normalized, None).unwrap_or_else(|| {
            tracing::error!("Graph structural parse failed for file: {}", path);
            std::process::exit(1);
        });
        let root = tree.root_node();
        let mut signals = Vec::new();
        self.extract_and_normalize(root, &normalized, &mut signals, path);

        // Determinism: sort signals by byte_start for stable ordering
        signals.sort_by_key(|s| s.location.byte_start);

        let mut graph = SymbolGraph::new();

        // Add nodes from signals
        for sig in &signals {
            graph.add_node(
                sig.symbol_id.clone(),
                &sig.symbol_kind,
                path,
                sig.location.start_line,
            );
        }

        // Extract call edges
        self.extract_call_graph(root, &normalized, path, &mut graph);

        (signals, graph)
    }

    /// Walk AST to extract call/import relationships.
    fn extract_call_graph(
        &self,
        node: Node,
        source: &str,
        _path: &str,
        graph: &mut crate::graph::SymbolGraph,
    ) {
        use crate::graph::EdgeType;

        let kind = node.kind();

        // Rust: call_expression → function()
        // Python: call → function()
        if kind == "call_expression" || kind == "call" {
            let caller = self.find_containing_function(node, source);
            let callee = self.extract_called_name(node, source);
            if let (Some(from), Some(to)) = (caller, callee) {
                graph.add_edge(&from, &to, EdgeType::Calls);
            }
        }

        // Rust: method_call_expression → object.method()
        if kind == "method_call_expression" {
            let caller = self.find_containing_function(node, source);
            if let Some(method_node) = node.child_by_field_name("name") {
                let method_name = method_node.utf8_text(source.as_bytes()).unwrap_or("");
                if let Some(from) = caller {
                    graph.add_edge(&from, method_name, EdgeType::Calls);
                }
            }
        }

        // Rust: use_statement → use module::function
        // Python: import_statement / import_from_statement
        if kind == "use_statement" || kind == "import_statement" || kind == "import_from_statement"
        {
            let container = self
                .find_containing_function(node, source)
                .unwrap_or_else(|| "_module".to_string());
            let import_name = node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
            if !import_name.is_empty() {
                graph.add_edge(&container, &import_name, EdgeType::Imports);
            }
        }

        // Recursive walk
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_call_graph(child, source, _path, graph);
        }
    }

    /// Walk up from a node to find the containing function name.
    fn find_containing_function(&self, node: Node, source: &str) -> Option<String> {
        let mut current = node.parent();
        while let Some(parent) = current {
            let kind = parent.kind();
            if (kind == "function_item"
                || kind == "function_definition"
                || kind == "async_function_definition")
                && let Some(name_node) = parent.child_by_field_name("name")
            {
                return Some(
                    name_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("unknown")
                        .to_string(),
                );
            }
            current = parent.parent();
        }
        None
    }

    /// Extract the name being called from a call_expression/call node.
    fn extract_called_name(&self, node: Node, source: &str) -> Option<String> {
        if let Some(func_node) = node.child_by_field_name("function") {
            return Some(
                func_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown")
                    .to_string(),
            );
        }
        // Fallback: first child
        if let Some(first) = node.child(0) {
            return Some(
                first
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown")
                    .to_string(),
            );
        }
        None
    }

    /// Phase A & B integrated traversal
    fn extract_and_normalize(
        &self,
        node: Node,
        source: &str,
        signals: &mut Vec<CognitiveSignal>,
        path: &str,
    ) {
        let kind = node.kind();

        // Phase A: Extraction (Finding the Anchor)
        // We ONLY look for anchors in comment nodes to avoid root-level swallowing or collision.
        if kind.contains("comment") || kind == "html_block" {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            if let Some(uuid) = Self::extract_uuid(text) {
                // Phase B: Normalization (Geometric Target -> Signal)
                if let Some(target_node) = self.find_geometric_target(node, source) {
                    let signal = self.map_to_signal(uuid, target_node, source, path);
                    signals.push(signal);
                }
            }
        }

        for child in node.children(&mut node.walk()) {
            self.extract_and_normalize(child, source, signals, path);
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

            if node.start_byte() >= offset
                && let Some(target) = self.as_solid_target(node)
            {
                return Some(target);
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

    fn map_to_signal(
        &self,
        uuid: String,
        node: Node,
        source: &str,
        file_path: &str,
    ) -> CognitiveSignal {
        let span = node.byte_range();
        let content = &source.as_bytes()[span.clone()];
        let kind_str = node.kind();

        // Determinism: normalize line endings (\r\n → \n) before hashing.
        // This ensures structural_hash is identical across Windows and Unix.
        let normalized_content: Vec<u8> = {
            let as_str = std::str::from_utf8(content).unwrap_or("");
            as_str.replace("\r\n", "\n").replace('\r', "\n").into_bytes()
        };

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
        let structural_hash = Self::compute_hash(&normalized_content);

        // Semantic Hash (whitespace-stripped, including normalized line endings)
        let semantic_content: Vec<u8> = normalized_content
            .iter()
            .filter(|&&b| !b.is_ascii_whitespace())
            .cloned()
            .collect();
        let semantic_hash = Self::compute_hash(&semantic_content);

        // Normalized AST Hash (identifier-stripped, rename-invariant)
        let normalized_ast = self.normalized_ast_string(node, source);
        let normalized_hash = Self::compute_hash(normalized_ast.as_bytes());

        // Source Location from tree-sitter
        let start_pos = node.start_position();
        let end_pos = node.end_position();
        let location = SourceLocation {
            file: file_path.to_string(),
            start_line: (start_pos.row + 1) as u32,
            start_col: start_pos.column as u32,
            end_line: (end_pos.row + 1) as u32,
            end_col: end_pos.column as u32,
            byte_start: span.start,
            byte_end: span.end,
        };

        CognitiveSignal {
            uuid,
            symbol_id,
            parent: None,
            symbol_kind,
            language: self.language_name.clone(),
            structural_hash,
            semantic_hash,
            normalized_hash,
            signature: None,
            location,
            metadata: HashMap::new(),
            origin: Origin {
                parser: format!("tree-sitter-{}", self.language_name),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            confidence: 1.0,
        }
    }

    /// Generate a normalized AST S-expression with identifiers stripped.
    /// This produces a rename-invariant hash: changing variable names
    /// does NOT change this hash, only structural changes do.
    fn normalized_ast_string(&self, node: Node, source: &str) -> String {
        let mut result = String::new();
        self.normalized_ast_walk(node, source, &mut result, 0);
        result
    }

    fn normalized_ast_walk(&self, node: Node, source: &str, result: &mut String, depth: usize) {
        let kind = node.kind();

        // Strip identifiers: replace actual names with token type
        if kind == "identifier" || kind == "type_identifier" {
            result.push_str(&format!("{}IDENT\n", "  ".repeat(depth)));
            return;
        }

        // Strip string literals
        if kind == "string_content" || kind == "string" {
            result.push_str(&format!("{}STR\n", "  ".repeat(depth)));
            return;
        }

        // Strip integers
        if kind == "integer_literal" {
            result.push_str(&format!("{}INT\n", "  ".repeat(depth)));
            return;
        }

        // Keep named structural nodes
        if node.is_named() {
            result.push_str(&format!("{}{}\n", "  ".repeat(depth), kind));
        } else if node.child_count() == 0 {
            // Anonymous leaf nodes: include operators (+, *, ==, etc.)
            // These are semantically critical but stripped of whitespace
            let text = node.utf8_text(source.as_bytes()).unwrap_or("").trim();
            if !text.is_empty() {
                result.push_str(&format!("{}{}\n", "  ".repeat(depth), text));
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.normalized_ast_walk(child, source, result, depth + 1);
        }
    }

    fn extract_symbol_id(&self, node: Node, source: &str) -> String {
        if let Some(id_node) = node.child_by_field_name("name") {
            return id_node
                .utf8_text(source.as_bytes())
                .unwrap_or("unknown")
                .to_string();
        }
        if let Some(id_node) = node.child_by_field_name("function") {
            return id_node
                .utf8_text(source.as_bytes())
                .unwrap_or("unknown")
                .to_string();
        }

        // Fallback for ERROR nodes: find the first identifier
        if node.kind() == "ERROR" {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "identifier" {
                    return child
                        .utf8_text(source.as_bytes())
                        .unwrap_or("unknown")
                        .to_string();
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

pub fn get_parser(lang: Language) -> Result<EpistemicParser, String> {
    match lang {
        Language::Rust => EpistemicParser::new_rust_parser(),
        Language::Python => EpistemicParser::new_python_parser(),
    }
}
