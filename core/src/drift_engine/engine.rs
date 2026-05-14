//! # Drift Graph Engine — v1.2.5
//!
//! Compares two UnifiedGraph snapshots and produces a structured drift report.
//! Pure deterministic comparison — no AI, no heuristics.
//!
//! ## What it detects
//! - Nodes added/removed between snapshots
//! - Intent annotations lost or changed
//! - Cross-language parity broken
//! - Invariant violations (reuses InvariantEngine)

use vantage_types::drift_graph::*;
use vantage_types::invariant_spec::InvariantSpec;
use vantage_types::uir::UnifiedGraph;
use crate::invariant_engine::engine as invariant_engine;

/// Compare two UnifiedGraph snapshots and produce a full drift report.
pub fn compare(
    baseline: &UnifiedGraph,
    current: &UnifiedGraph,
    invariants: &[InvariantSpec],
) -> GraphDriftReport {
    // 1. Node-level diff (added/removed)
    let (added_nodes, removed_nodes) = diff_nodes(&baseline.nodes, &current.nodes);

    // 2. Intent drift
    let intent_breaks = diff_intents(&baseline.nodes, &current.nodes);

    // 3. Cross-language parity drift
    let parity_breaks = diff_parity(&baseline.nodes, &current.nodes);

    // 4. Invariant validation on current graph
    let inv_report = invariant_engine::validate_invariants(current, invariants);
    let invariant_breaks: Vec<InvariantBreak> = inv_report
        .results
        .into_iter()
        .filter(|r| !r.passed)
        .map(|r| InvariantBreak {
            spec_id: r.spec_id,
            description: r.message,
            node: None,
        })
        .collect();

    let has_drift = !added_nodes.is_empty()
        || !removed_nodes.is_empty()
        || !intent_breaks.is_empty()
        || !parity_breaks.is_empty()
        || !invariant_breaks.is_empty();

    GraphDriftReport {
        schema_version: vantage_types::DRIFT_SCHEMA_VERSION.to_string(),
        added_nodes,
        removed_nodes,
        changed_edges: Vec::new(), // edge-level diff is an enhancement for later
        invariant_breaks: [intent_breaks, invariant_breaks].concat(),
        parity_breaks,
        baseline_node_count: baseline.nodes.len(),
        current_node_count: current.nodes.len(),
        has_drift,
    }
}

/// Quick drift check — runs compare with default invariants.
pub fn quick_check(baseline: &UnifiedGraph, current: &UnifiedGraph) -> GraphDriftReport {
    compare(baseline, current, &[])
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_types::uir::{Language, UnifiedNode, Visibility};
    use vantage_types::SymbolId;
    use vantage_types::intent::IntentInvariant;

    fn make_node(fq_name: &str, lang: Language, kind: vantage_types::symbol::SymbolKind) -> UnifiedNode {
        UnifiedNode {
            id: SymbolId::new(fq_name),
            fq_name: fq_name.to_string(),
            language: lang,
            kind,
            visibility: Visibility::Public,
            dependencies: vec![],
            file: "test.rs".to_string(),
            line: 1,
            structural_hash: "abc".to_string(),
            normalized_hash: "abc".to_string(),
            intent: None,
        }
    }

    #[test]
    fn test_no_drift_identical_graphs() {
        let nodes = vec![make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct)];
        let g = UnifiedGraph { nodes: nodes.clone(), source_language: Language::Rust };
        let report = quick_check(&g, &g);
        assert!(!report.has_drift);
        assert!(report.added_nodes.is_empty());
        assert!(report.removed_nodes.is_empty());
    }

    #[test]
    fn test_drift_node_added() {
        let baseline = UnifiedGraph {
            nodes: vec![make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct)],
            source_language: Language::Rust,
        };
        let current = UnifiedGraph {
            nodes: vec![
                make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct),
                make_node("B", Language::Python, vantage_types::symbol::SymbolKind::Class),
            ],
            source_language: Language::Rust,
        };
        let report = quick_check(&baseline, &current);
        assert!(report.has_drift);
        assert_eq!(report.added_nodes.len(), 1);
        assert_eq!(report.added_nodes[0].fq_name, "B");
    }

    #[test]
    fn test_drift_node_removed() {
        let baseline = UnifiedGraph {
            nodes: vec![
                make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct),
                make_node("B", Language::Python, vantage_types::symbol::SymbolKind::Class),
            ],
            source_language: Language::Rust,
        };
        let current = UnifiedGraph {
            nodes: vec![make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct)],
            source_language: Language::Rust,
        };
        let report = quick_check(&baseline, &current);
        assert!(report.has_drift);
        assert_eq!(report.removed_nodes.len(), 1);
        assert_eq!(report.removed_nodes[0].fq_name, "B");
    }

    #[test]
    fn test_intent_drift_detected() {
        let mut node_with_intent = make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct);
        node_with_intent.intent = Some(IntentInvariant::AppendOnly).map(|inv| {
            vantage_types::intent::IntentOverlay {
                invariant: inv,
                reason: "test".to_string(),
                constraints: vec![],
                owner: None,
                metadata: std::collections::HashMap::new(),
            }
        });

        let baseline = UnifiedGraph {
            nodes: vec![node_with_intent.clone()],
            source_language: Language::Rust,
        };

        // Current graph has no intent on the node
        let mut node_no_intent = make_node("A", Language::Rust, vantage_types::symbol::SymbolKind::Struct);
        node_no_intent.structural_hash = "abc".to_string();
        let current = UnifiedGraph {
            nodes: vec![node_no_intent],
            source_language: Language::Rust,
        };

        let report = quick_check(&baseline, &current);
        assert!(report.has_drift);
        assert!(!report.invariant_breaks.is_empty());
    }

    #[test]
    fn test_parity_drift_detected() {
        // Both languages had Arena
        let baseline = UnifiedGraph {
            nodes: vec![
                make_node("core::Arena", Language::Rust, vantage_types::symbol::SymbolKind::Struct),
                make_node("core.Arena", Language::Python, vantage_types::symbol::SymbolKind::Class),
            ],
            source_language: Language::Rust,
        };

        // Python Arena was removed
        let current = UnifiedGraph {
            nodes: vec![
                make_node("core::Arena", Language::Rust, vantage_types::symbol::SymbolKind::Struct),
                // Python Arena removed — should trigger parity break
            ],
            source_language: Language::Rust,
        };

        let report = quick_check(&baseline, &current);
        assert!(report.has_drift);
        assert!(!report.parity_breaks.is_empty());
    }
}
