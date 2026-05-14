//! # Graph Drift Report — v1.2.5
//!
//! Deterministic topology diff between two UnifiedGraph snapshots.
//! Detects structural erosion, intent loss, and parity drift.
//! Pure structural comparison — no AI, no vectors, no heuristics.

use serde::{Deserialize, Serialize};
use crate::symbol::SymbolKind;
use crate::uir::{Language, UnifiedNode};
use crate::version::DRIFT_SCHEMA_VERSION;

/// A node that was added or removed between two graph snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDelta {
    pub fq_name: String,
    pub language: Language,
    pub kind: SymbolKind,
    pub file: String,
    pub line: u32,
}

/// A dependency edge that changed between two graph snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDelta {
    pub source: String,
    pub target: String,
    pub change: EdgeChangeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeChangeKind {
    Added,
    Removed,
}

/// An architectural invariant that was broken.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantBreak {
    pub spec_id: String,
    pub description: String,
    pub node: Option<String>,
}

/// Cross-language parity drift — a symbol that exists in multiple languages
/// but has diverged in structure or intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParityBreak {
    pub symbol_name: String,
    pub languages: Vec<Language>,
    pub description: String,
}

/// Complete drift report between two graph snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDriftReport {
    /// Version of the drift schema
    pub schema_version: String,
    /// Nodes present in current but not in baseline
    pub added_nodes: Vec<NodeDelta>,
    /// Nodes present in baseline but not in current
    pub removed_nodes: Vec<NodeDelta>,
    /// Dependency edges that changed
    pub changed_edges: Vec<EdgeDelta>,
    /// Architectural invariants that no longer hold
    pub invariant_breaks: Vec<InvariantBreak>,
    /// Cross-language parity that drifted
    pub parity_breaks: Vec<ParityBreak>,
    /// Snapshot metadata
    pub baseline_node_count: usize,
    pub current_node_count: usize,
    pub has_drift: bool,
}

impl GraphDriftReport {
    pub fn empty() -> Self {
        Self {
            schema_version: DRIFT_SCHEMA_VERSION.to_string(),
            added_nodes: Vec::new(),
            removed_nodes: Vec::new(),
            changed_edges: Vec::new(),
            invariant_breaks: Vec::new(),
            parity_breaks: Vec::new(),
            baseline_node_count: 0,
            current_node_count: 0,
            has_drift: false,
        }
    }
}

fn node_key(node: &UnifiedNode) -> String {
    format!("{}:{}", node.language.as_str(), node.fq_name)
}

/// Compute the set of added and removed nodes between two graphs.
pub fn diff_nodes(baseline: &[UnifiedNode], current: &[UnifiedNode]) -> (Vec<NodeDelta>, Vec<NodeDelta>) {
    let baseline_keys: std::collections::HashSet<String> = baseline.iter().map(node_key).collect();
    let current_keys: std::collections::HashSet<String> = current.iter().map(node_key).collect();

    let mut added: Vec<NodeDelta> = current
        .iter()
        .filter(|n| !baseline_keys.contains(&node_key(n)))
        .map(|n| NodeDelta {
            fq_name: n.fq_name.clone(),
            language: n.language,
            kind: n.kind.clone(),
            file: n.file.clone(),
            line: n.line,
        })
        .collect();

    let mut removed: Vec<NodeDelta> = baseline
        .iter()
        .filter(|n| !current_keys.contains(&node_key(n)))
        .map(|n| NodeDelta {
            fq_name: n.fq_name.clone(),
            language: n.language,
            kind: n.kind.clone(),
            file: n.file.clone(),
            line: n.line,
        })
        .collect();

    added.sort_by(|a, b| a.language.as_str().cmp(b.language.as_str()).then(a.fq_name.cmp(&b.fq_name)));
    removed.sort_by(|a, b| a.language.as_str().cmp(b.language.as_str()).then(a.fq_name.cmp(&b.fq_name)));

    (added, removed)
}

/// Detect intent drift — nodes that lost their intent annotation.
pub fn diff_intents(baseline: &[UnifiedNode], current: &[UnifiedNode]) -> Vec<InvariantBreak> {
    let current_intents: std::collections::HashMap<String, &UnifiedNode> = current
        .iter()
        .filter(|n| n.intent.is_some())
        .map(|n| (node_key(n), n))
        .collect();

    let mut breaks = Vec::new();

    for node in baseline {
        if let Some(ref intent) = node.intent {
            let key = node_key(node);
            if let Some(current_node) = current_intents.get(&key) {
                if current_node.intent.as_ref().map(|i| &i.invariant) != Some(&intent.invariant) {
                    breaks.push(InvariantBreak {
                        spec_id: "intent-drift".to_string(),
                        description: format!(
                            "{} lost {:?} intent (was {:?}, now {:?})",
                            node.fq_name,
                            intent.invariant,
                            intent.invariant,
                            current_node.intent.as_ref().map(|i| &i.invariant),
                        ),
                        node: Some(node.fq_name.clone()),
                    });
                }
            } else {
                // Node was removed entirely
                breaks.push(InvariantBreak {
                    spec_id: "intent-drift".to_string(),
                    description: format!("{} with {:?} intent was removed", node.fq_name, intent.invariant),
                    node: Some(node.fq_name.clone()),
                });
            }
        }
    }

    breaks.sort_by(|a, b| a.spec_id.cmp(&b.spec_id).then(a.node.cmp(&b.node)));
    breaks
}

/// Extract the base symbol name from an fq_name regardless of separator conventions.
fn base_name(fq_name: &str) -> &str {
    // Handle Rust-style (::), Python-style (.), and Ruby-style (::) separators
    for sep in &["::", "."] {
        if let Some(name) = fq_name.rsplit(sep).next() {
            if name != fq_name {
                return name;
            }
        }
    }
    fq_name
}

/// Detect cross-language parity drift.
pub fn diff_parity(baseline: &[UnifiedNode], current: &[UnifiedNode]) -> Vec<ParityBreak> {
    // Group nodes by base symbol name (strip language prefix)
    fn group_by_name<'a>(nodes: &'a [UnifiedNode]) -> std::collections::HashMap<String, Vec<&'a UnifiedNode>> {
        let mut map: std::collections::HashMap<String, Vec<&UnifiedNode>> = std::collections::HashMap::new();
        for node in nodes {
            let name = base_name(&node.fq_name).to_string();
            map.entry(name).or_default().push(node);
        }
        map
    }

    let baseline_groups = group_by_name(baseline);
    let current_groups = group_by_name(current);

    let mut breaks = Vec::new();

    for (name, baseline_nodes) in &baseline_groups {
        if let Some(current_nodes) = current_groups.get(name) {
            let baseline_langs: std::collections::HashSet<Language> =
                baseline_nodes.iter().map(|n| n.language).collect();
            let current_langs: std::collections::HashSet<Language> =
                current_nodes.iter().map(|n| n.language).collect();

            // Check if any language was lost
            let lost: Vec<&Language> = baseline_langs.difference(&current_langs).collect();
            if !lost.is_empty() {
                let lost_str: Vec<String> = lost.iter().map(|l| l.as_str().to_string()).collect();
                breaks.push(ParityBreak {
                    symbol_name: name.to_string(),
                    languages: lost.into_iter().cloned().collect(),
                    description: format!("'{}' lost in languages: {}", name, lost_str.join(", ")),
                });
            }
        }
    }

    breaks.sort_by(|a, b| a.symbol_name.cmp(&b.symbol_name));
    breaks
}
