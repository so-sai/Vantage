use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tree_sitter::{Node, Parser};
use vantage_types::semantic_role::SemanticRole;
use vantage_types::{CafBuilder, CafContext, CognitiveSignal, DefaultAlgebraResolver, IdentityAnchor,
    NodeArena, NodeId, Origin, PerfMetrics, RoleResolver,
    SourceLocation, SymbolKind, SymbolScopeRegistry, DOMAIN_ROOT,
    SymbolId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Rust,
    Python,
    Ruby,
    Javascript,
    Typescript,
    Tsx,
}

impl Language {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "rb" => Some(Language::Ruby),
            "js" | "jsx" => Some(Language::Javascript),
            "ts" => Some(Language::Typescript),
            "tsx" => Some(Language::Tsx),
            _ => None,
        }
    }
}

pub struct EpistemicParser {
    parser: Parser,
    language_name: String,
    solid_kinds: Vec<String>,
    algebra_resolver: DefaultAlgebraResolver,
    pub arena: NodeArena,
    pub symbol_registry: SymbolScopeRegistry,
    pub metrics: PerfMetrics,
    node_id_map: HashMap<usize, NodeId>,
    pub caf_context: CafContext,
    solid_node_index: Vec<(usize, usize, u8)>,
    pub language: Language,
}

impl EpistemicParser {
    pub fn new(lang_name: &str, solid_kinds: Vec<&str>) -> Result<Self, String> {
        let mut parser = Parser::new();
        let (lang, language) = match lang_name {
            "rust" => (tree_sitter_rust::LANGUAGE.into(), Language::Rust),
            "python" => (tree_sitter_python::LANGUAGE.into(), Language::Python),
            "ruby" => (tree_sitter_ruby::LANGUAGE.into(), Language::Ruby),
            "javascript" => (tree_sitter_javascript::LANGUAGE.into(), Language::Javascript),
            "typescript" => (tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), Language::Typescript),
            "tsx" => (tree_sitter_typescript::LANGUAGE_TSX.into(), Language::Tsx),
            _ => return Err(format!("Unsupported language: {}", lang_name)),
        };

        parser
            .set_language(&lang)
            .map_err(|_| "Failed to load language")?;

        Ok(Self {
            parser,
            language,
            language_name: lang_name.to_string(),
            solid_kinds: solid_kinds.iter().map(|s| s.to_string()).collect(),
            algebra_resolver: DefaultAlgebraResolver::new(),
            arena: NodeArena::new(),
            symbol_registry: SymbolScopeRegistry::new(),
            metrics: PerfMetrics::new(),
            node_id_map: HashMap::new(),
            caf_context: CafContext::new(),
            solid_node_index: Vec::new(),
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

    pub fn new_ruby_parser() -> Result<Self, String> {
        Self::new(
            "ruby",
            vec![
                "method",
                "class",
                "module",
            ],
        )
    }

    pub fn new_javascript_parser() -> Result<Self, String> {
        Self::new(
            "javascript",
            vec![
                "function_declaration",
                "arrow_function",
                "call_expression",
            ],
        )
    }

    pub fn new_typescript_parser() -> Result<Self, String> {
        Self::new(
            "typescript",
            vec![
                "function_declaration",
                "method_definition",
                "interface_declaration",
                "type_alias_declaration",
            ],
        )
    }

    pub fn new_tsx_parser() -> Result<Self, String> {
        Self::new(
            "tsx",
            vec![
                "function_declaration",
                "jsx_element",
                "jsx_opening_element",
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
            std::process::exit(1); 
        });
        let root = tree.root_node();
        
        // Build solid node index once (O(n)) for binary search lookup (O(log n) per tag)
        self.build_solid_node_index(root);
        
        let mut signals = Vec::new();

        // 1. Reset Transient State
        self.node_id_map.clear();
        self.metrics.bump();
        self.arena.bump_generation();
        self.symbol_registry = SymbolScopeRegistry::new();
        self.caf_context = CafContext::new(); // Identity physics reset per pass

        // 2. Perform Full Incremental CAF Walk (v1.2.4 Formula)
        // Root anchored to DOMAIN_ROOT for workspace stability.
        self.compute_caf_hash(root, DOMAIN_ROOT, 0, &normalized);

        // 3. Extract Anchors and Map to Signals
        self.extract_and_normalize(root, &normalized, &mut signals, path);

        // 4. Persistence GC (Evict nodes not seen in this pass)
        self.arena.gc();

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
    ) -> (Vec<CognitiveSignal>, crate::SymbolDependencyGraph) {
        use crate::SymbolDependencyGraph;

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
        
        // Build solid node index once (O(n)) for binary search lookup (O(log n) per tag)
        self.build_solid_node_index(root);
        
        let mut signals = Vec::new();
        self.extract_and_normalize(root, &normalized, &mut signals, path);
        signals.sort_by_key(|s| s.location.byte_start);

        let mut graph = SymbolDependencyGraph::new();
        
        // Actually walk the tree to find calls!
        self.extract_call_graph(root, &normalized, path, &mut graph);

        // Add nodes from signals
        for sig in &signals {
            graph.add_node(
                sig.symbol_id.clone(),
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
        graph: &mut crate::SymbolDependencyGraph,
    ) {
        use vantage_types::graph::DependencyKind;

        let kind = node.kind();
        let caller = self.find_containing_function(node, source);

        // Ruby: call, method_call, command
        if self.language == Language::Ruby {
            if kind == "call" || kind == "method_call" || kind == "command" {
                let from = caller.clone().unwrap_or_else(|| vantage_types::SymbolId::new("crate"));
                if let Some(to) = self.extract_called_name(node, source) {
                    graph.add_edge(&from, &to, DependencyKind::CallEdge);
                    
                    // Capture symbol arguments as potential dependencies
                    // e.g. before_action :authenticate_user!
                    for i in 0..node.child_count() {
                        let child = node.child(i).unwrap();
                        if child.kind() == "argument_list" {
                            for j in 0..child.child_count() {
                                let arg = child.child(j).unwrap();
                                if arg.kind() == "simple_symbol" || arg.kind() == "symbol" {
                                    let sym_text = arg.utf8_text(source.as_bytes()).unwrap_or("").trim_start_matches(':');
                                    if !sym_text.is_empty() {
                                        graph.add_edge(&from, &vantage_types::SymbolId::new(sym_text), DependencyKind::CallEdge);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // JS/TS: call_expression, jsx_opening_element, jsx_self_closing_element
        if self.language == Language::Javascript || self.language == Language::Typescript || self.language == Language::Tsx {
            if kind == "call_expression" || kind == "jsx_opening_element" || kind == "jsx_self_closing_element" {
                if let Some(target) = self.extract_called_name(node, source) {
                    // Filter out intrinsic tags for JSX (e.g., <div>, <span>)
                    if kind.starts_with("jsx_") {
                        let name = target.to_string();
                        // React components must start with an uppercase letter
                        if !name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                            return;
                        }
                    }
                    let from = caller.clone().unwrap_or_else(|| vantage_types::SymbolId::new("crate"));
                    graph.add_edge(&from, &target, DependencyKind::CallEdge);
                }
            }
        }

        // Rust: call_expression → function()
        // Python: call → function()
        if kind == "call_expression" || kind == "call" {
            let from = caller.clone().unwrap_or_else(|| vantage_types::SymbolId::new("crate"));
            if let Some(to) = self.extract_called_name(node, source) {
                graph.add_edge(&from, &to, DependencyKind::CallEdge);
            }
        }

        // Python Decorators: @app.route()
        if self.language == Language::Python && kind == "decorator" {
            // Find the decorated function to use as source
            let mut from = vantage_types::SymbolId::new("crate");
            if let Some(parent) = node.parent() {
                if parent.kind() == "decorated_definition" {
                    for i in 0..parent.child_count() {
                        let child = parent.child(i).unwrap();
                        if child.kind() == "function_definition" {
                            if let Some(name_node) = child.child_by_field_name("name") {
                                from = SymbolId::new(name_node.utf8_text(source.as_bytes()).unwrap_or("unknown"));
                                break;
                            }
                        }
                    }
                }
            }
            if let Some(to) = self.extract_called_name(node, source) {
                graph.add_edge(&from, &to, DependencyKind::CallEdge);
            }
        }

        // Rust: method_call_expression → object.method()
        if kind == "method_call_expression" {
            let caller = self.find_containing_function(node, source);
            if let Some(method_node) = node.child_by_field_name("name") {
                let method_name = method_node.utf8_text(source.as_bytes()).unwrap_or("");
                if let Some(from) = caller
                    && !method_name.is_empty()
                {
                    let to = SymbolId::new(method_name);
                    graph.add_edge(&from, &to, DependencyKind::CallEdge);
                }
            }
        }

        // Rust: use_statement → use module::function
        // Python: import_statement / import_from_statement
        if kind == "use_statement" || kind == "import_statement" || kind == "import_from_statement"
        {
            let container = self
                .find_containing_function(node, source)
                .unwrap_or_else(SymbolId::root);
            let import_name = node
                .utf8_text(source.as_bytes())
                .unwrap_or("")
                .trim()
                .to_string();
            if !import_name.is_empty() {
                let to = vantage_types::SymbolId::new(&import_name);
                graph.add_edge(&container, &to, DependencyKind::ModuleImport);
            }
        }

        // Recursive walk
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_call_graph(child, source, _path, graph);
        }
    }

    /// Walk up from a node to find the containing function name as a SymbolId.
    fn find_containing_function(&self, node: Node, source: &str) -> Option<SymbolId> {
        let mut current = node.parent();
        while let Some(parent) = current {
            let kind = parent.kind();
            if (kind == "function_item"
                || kind == "function_definition"
                || kind == "async_function_definition"
                || kind == "function_declaration"
                || kind == "method_definition")
                && let Some(name_node) = parent.child_by_field_name("name")
            {
                return Some(
                    SymbolId::new(name_node
                        .utf8_text(source.as_bytes())
                        .unwrap_or("unknown"))
                );
            }
            current = parent.parent();
        }
        None
    }

    /// Extract the name being called from a call_expression/call node as a SymbolId.
    fn extract_called_name(&self, node: Node, source: &str) -> Option<SymbolId> {
        if let Some(func_node) = node.child_by_field_name("function") {
            return Some(
                SymbolId::new(func_node
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown"))
            );
        }

        // Ruby: method_call (method field), call (method field), command (method field)
        if self.language == Language::Ruby {
            let method = node.child_by_field_name("method");
            let receiver = node.child_by_field_name("receiver");

            if let Some(m) = method {
                let m_text = m.utf8_text(source.as_bytes()).unwrap_or("unknown");
                if let Some(r) = receiver {
                    let r_text = r.utf8_text(source.as_bytes()).unwrap_or("");
                    return Some(SymbolId::new(&format!("{}.{}", r_text, m_text)));
                } else {
                    return Some(SymbolId::new(m_text));
                }
            }

            // Fallback: If no method field, look for identifier/constant children
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                let c_kind = child.kind();
                if c_kind == "identifier" || c_kind == "constant" {
                    return Some(SymbolId::new(child.utf8_text(source.as_bytes()).unwrap_or("unknown")));
                }
            }
        }

        // JS/TS: call_expression (function field), jsx_opening_element/jsx_self_closing_element (name field)
        if self.language == Language::Javascript || self.language == Language::Typescript || self.language == Language::Tsx {
            if let Some(name_node) = node.child_by_field_name("name") {
                return Some(SymbolId::new(name_node.utf8_text(source.as_bytes()).unwrap_or("unknown")));
            }
            if let Some(func_node) = node.child_by_field_name("function") {
                return Some(SymbolId::new(func_node.utf8_text(source.as_bytes()).unwrap_or("unknown")));
            }
            
            // Fallback: Use the first identifier/member_expression child (common for JSX and simple calls)
            for i in 0..node.child_count() {
                let child = node.child(i).unwrap();
                let c_kind = child.kind();
                if c_kind == "identifier" || c_kind == "member_expression" || c_kind == "property_identifier" || c_kind == "jsx_member_expression" {
                    return Some(SymbolId::new(child.utf8_text(source.as_bytes()).unwrap_or("unknown")));
                }
            }
        }
        
        // Python Decorator target
        if self.language == Language::Python && node.kind() == "decorator" {
             // Decorator usually has an identifier or call child
             for i in 0..node.child_count() {
                 let child = node.child(i).unwrap();
                 if child.kind() == "identifier" || child.kind() == "call" || child.kind() == "attribute" {
                     return Some(SymbolId::new(child.utf8_text(source.as_bytes()).unwrap_or("unknown")));
                 }
             }
        }
        // Fallback: first child
        if let Some(first) = node.child(0) {
            return Some(
                SymbolId::new(first
                    .utf8_text(source.as_bytes())
                    .unwrap_or("unknown"))
            );
        }
        None
    }

    /// Phase A & B integrated traversal
    fn extract_and_normalize(
        &mut self,
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
                if let Some((target_start, target_end, target_role)) = self.find_geometric_target(node, source) {
                    let target_kind = Self::role_to_kind(target_role);
                    let signal = self.map_to_signal_from_offset(uuid, target_start, target_end, target_kind, source, path);
                    signals.push(signal);
                }
            }
        }

        for child in node.children(&mut node.walk()) {
            self.extract_and_normalize(child, source, signals, path);
        }
    }

    fn map_to_signal_from_offset(
        &mut self,
        uuid: String,
        byte_start: usize,
        byte_end: usize,
        kind_str: &str,
        source: &str,
        file_path: &str,
    ) -> CognitiveSignal {
        let span = byte_start..byte_end;
        let content = &source.as_bytes()[span.clone()];

        // Determinism: normalize line endings (\r\n → \n) before hashing.
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

        // Extract actual symbol name from source
        let symbol_id = Self::extract_name_from_node(source, byte_start, byte_end, kind_str);
        
        let structural_hash = Self::compute_hash(&normalized_content);

        // Semantic Hash
        let semantic_content: Vec<u8> = normalized_content
            .iter()
            .filter(|&&b| !b.is_ascii_whitespace())
            .cloned()
            .collect();
        let semantic_hash = Self::compute_hash(&semantic_content);

        // CAF hash - use placeholder since we don't have node access
        let _caf_hash = "INDEXED_CAF".to_string();

        // Normalized hash
        let normalized_str = String::from_utf8_lossy(&normalized_content);
        let normalized_content_for_hash: String = normalized_str
            .chars()
            .filter(|c| !c.is_alphabetic() && *c != '_')
            .collect();
        let normalized_hash = Self::compute_hash(normalized_content_for_hash.as_bytes());

        // Calculate line/column from byte offset
        let (start_line, start_col) = Self::byte_to_line_col(source, byte_start);
        let (end_line, end_col) = Self::byte_to_line_col(source, byte_end);

        let location = SourceLocation {
            file: file_path.to_string(),
            start_line,
            start_col,
            end_line,
            end_col,
            byte_start,
            byte_end,
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

    fn byte_to_line_col(source: &str, byte_offset: usize) -> (u32, u32) {
        let mut line = 1u32;
        let mut col = 1u32;
        for (i, c) in source.char_indices() {
            if i >= byte_offset {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    pub fn find_geometric_target(&mut self, anchor: Node, _source: &str) -> Option<(usize, usize, u8)> {
        self.find_first_solid_after_byte(anchor.end_byte())
    }


    fn build_solid_node_index(&mut self, root: Node) {
        self.solid_node_index.clear();
        self.collect_solid_nodes(root);
        self.solid_node_index.sort_by_key(|(offset, _, _)| *offset);
    }

    fn collect_solid_nodes(&mut self, node: Node) {
        if let Some(kind_str) = self.solid_kinds.iter().find(|k| *k == node.kind())
            && node.is_named()
        {
            let role = Self::kind_to_role(kind_str);
            self.solid_node_index.push((
                node.start_byte(),
                node.end_byte(),
                role as u8,
            ));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_solid_nodes(child);
        }
    }

    fn kind_to_role(kind: &str) -> SemanticRole {
        match kind {
            "function_item" | "function_definition" | "async_function_definition" => SemanticRole::FunctionName,
            "struct_item" => SemanticRole::SyntaxNode,
            "enum_item" => SemanticRole::SyntaxNode,
            "trait_item" => SemanticRole::SyntaxNode,
            "impl_item" => SemanticRole::SyntaxNode,
            "mod_item" => SemanticRole::ModuleItem,
            "const_item" | "static_item" => SemanticRole::SymbolDeclaration,
            "macro_invocation" => SemanticRole::SyntaxNode,
            "expression_statement" => SemanticRole::Statement,
            "call_expression" => SemanticRole::CallTarget,
            _ => SemanticRole::SyntaxNode,
        }
    }

    fn role_to_kind(role: u8) -> &'static str {
        match role as u16 {
            10 => "function_item",
            999 => "syntax_node",
            71 => "mod_item",
            32 => "symbol_declaration",
            21 => "statement",
            41 => "call_expression",
            _ => "unknown",
        }
    }

    fn find_first_solid_after_byte(&self, offset: usize) -> Option<(usize, usize, u8)> {
        // Find first node where start_byte > offset (next solid after anchor)
        let idx = self.solid_node_index.partition_point(|(start, _, _)| *start <= offset);
        
        if idx < self.solid_node_index.len() {
            let (start, end, role) = self.solid_node_index[idx];
            return Some((start, end, role));
        }
        None
    }


    /// Compute CAF hash using commutativity-aware hashing with scope context
    /// Compute CAF hash with O(depth) incremental support.
    /// Strictly enforces child-first recursion and parent-anchored NodeId.
    fn compute_caf_hash(
        &mut self,
        node: Node,
        parent_id: NodeId,
        depth: u16,
        source: &str,
    ) -> NodeId {
        self.metrics.update_depth(depth);

        // 1. Resolve Semantic Role
        let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("root");
        let role = RoleResolver::resolve(node.kind(), parent_kind, node.start_byte());

        // 2. Build Identity Anchor
        let anchor = self.build_anchor(node, source);

        // 3. Generate hardened NodeId (v1.2.4 Formula)
        let (node_id, _fingerprint) = NodeId::generate(Some(parent_id), role, &anchor);

        // 4. Cache Lookup - O(1) in HashMap
        if let Some(entry) = self.arena.get_mut_with_bump(&node_id) {
            // Check for EXACT structural hit (HashVersion check can be added here)
            if !entry.stamp.dirty {
                self.metrics.record_reuse();
                // MUST still map this ID for signal extraction in THIS pass
                self.node_id_map.insert(node.start_byte(), node_id);
                return node_id;
            }
        }

        self.metrics.record_recompute();

        // 5. Child-First Recursion
        // Manage logical scope transitions (e.g. into blocks, functions)
        let needs_new_scope = node.kind() == "block" || node.kind() == "function_item" || node.kind() == "class_definition";
        if needs_new_scope {
            self.caf_context.push_scope();
        }

        let mut child_hashes = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let child_id = self.compute_caf_hash(child, node_id, depth + 1, source);
            
            // Collect hashes from arena for CafBuilder
            if let Some(entry) = self.arena.get(&child_id) {
                child_hashes.push(entry.stamp.hash.clone());
            }
        }

        if needs_new_scope {
            self.caf_context.pop_scope();
        }

        // 6. Finalize Node and Hash
        let mut builder = CafBuilder::new(&self.algebra_resolver, &self.language_name)
            .with_scope(self.caf_context.clone()); // FIX: Use real scope context

        let (caf_node, caf_hash) = builder.build(
            node.kind(),
            child_hashes,
            node.start_byte(),
            node.end_byte(),
            Some(parent_id),
        );

        // 7. Store in Arena (Set-Once Invariant)
        let stamp = vantage_types::NodeStamp::new(caf_hash, 0); // epoch 0 for now
        self.arena.insert(node_id, caf_node, stamp);
        self.node_id_map.insert(node.start_byte(), node_id);

        node_id
    }

    fn build_anchor(&mut self, node: Node, source: &str) -> IdentityAnchor {
        let kind = node.kind();
        
        // Bindings (Functions, Variables, Types)
        if kind == "identifier" || kind == "type_identifier" || kind == "function_item" || kind == "function_definition" {
            let name = node.utf8_text(source.as_bytes()).unwrap_or("unknown");
            let symbol_id = self.symbol_registry.discover(name);
            return IdentityAnchor::Binding(symbol_id);
        }

        // Literals
        if kind.contains("literal") || kind == "string" || kind == "integer" {
            let text = node.utf8_text(source.as_bytes()).unwrap_or("");
            let hash = Self::compute_hash(text.as_bytes());
            return IdentityAnchor::Literal(hash);
        }

        // Operators
        if node.child_count() == 0 && !node.is_named() {
            let op = node.utf8_text(source.as_bytes()).unwrap_or("");
            if !op.trim().is_empty() {
                return IdentityAnchor::Operator(op.to_string(), 0);
            }
        }

        // Default: Structural node
        IdentityAnchor::Structural
    }



    fn extract_name_from_node(source: &str, byte_start: usize, byte_end: usize, kind: &str) -> SymbolId {
        // For solid nodes we found via index, we have the exact byte range
        // Extract the name by looking for identifier after the keyword
        let node_content = &source[byte_start..byte_end];
        
        // Skip keywords like "fn", "struct", "enum", etc. and find the actual name
        // These keywords are followed by whitespace then the name
        let content_chars: Vec<char> = node_content.chars().collect();
        
        let mut i = 0;
        // Skip the keyword
        while i < content_chars.len() {
            let c = content_chars[i];
            if c.is_whitespace() {
                break;
            }
            i += 1;
        }
        
        // Skip whitespace
        while i < content_chars.len() && content_chars[i].is_whitespace() {
            i += 1;
        }
        
        // Now we should be at the identifier
        let name_start = i;
        while i < content_chars.len() {
            let c = content_chars[i];
            if !c.is_alphanumeric() && c != '_' {
                break;
            }
            i += 1;
        }
        
        let name_end = i;
        
        if name_end > name_start {
            let name = &node_content[name_start..name_end];
            if !name.is_empty() && (name.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false) || name.starts_with('_')) {
                return SymbolId::new(name);
            }
        }
        
        // Fallback: use kind
        SymbolId::new(kind)
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
        Language::Ruby => EpistemicParser::new_ruby_parser(),
        Language::Javascript => EpistemicParser::new_javascript_parser(),
        Language::Typescript => EpistemicParser::new_typescript_parser(),
        Language::Tsx => EpistemicParser::new_tsx_parser(),
    }
}
