use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vantage_types::symbol::SymbolKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// A calls B (call_expression)
    Calls,
    /// A imports B (use_statement, import_statement)
    Imports,
    /// A uses B (field_expression, variable reference)
    Uses,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolNode {
    pub symbol_id: String,
    pub kind: String,
    pub file: String,
    pub start_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolGraph {
    pub nodes: HashMap<String, SymbolNode>,
    pub edges: Vec<Edge>,
}

impl SymbolGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, symbol_id: String, kind: &SymbolKind, file: &str, line: u32) {
        self.nodes.entry(symbol_id.clone()).or_insert(SymbolNode {
            symbol_id,
            kind: format!("{:?}", kind),
            file: file.to_string(),
            start_line: line,
        });
    }

    pub fn add_edge(&mut self, from: &str, to: &str, edge_type: EdgeType) {
        if from != to {
            self.edges.push(Edge {
                from: from.to_string(),
                to: to.to_string(),
                edge_type,
            });
        }
    }

    /// Impact radius: given a changed symbol, return all symbols that depend on it
    pub fn impact_radius(&self, symbol_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.to == symbol_id)
            .map(|e| e.from.as_str())
            .collect()
    }

    /// Downstream: given a changed symbol, return all symbols it depends on
    pub fn downstream(&self, symbol_id: &str) -> Vec<&str> {
        self.edges
            .iter()
            .filter(|e| e.from == symbol_id)
            .map(|e| e.to.as_str())
            .collect()
    }

    /// Determinism: return edges sorted by (from, to, edge_type) for stable output ordering.
    pub fn sorted_edges(&self) -> Vec<&Edge> {
        let mut sorted: Vec<&Edge> = self.edges.iter().collect();
        sorted.sort_by(|a, b| {
            a.from
                .cmp(&b.from)
                .then(a.to.cmp(&b.to))
                .then(format!("{:?}", a.edge_type).cmp(&format!("{:?}", b.edge_type)))
        });
        sorted
    }

    pub fn merge(&mut self, other: SymbolGraph) {
        for (id, node) in other.nodes {
            self.nodes.entry(id).or_insert(node);
        }
        self.edges.extend(other.edges);
    }
}

impl Default for SymbolGraph {
    fn default() -> Self {
        Self::new()
    }
}
