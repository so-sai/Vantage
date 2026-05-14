//! # Invariant Engine — v1.2.5
//!
//! Validates architectural invariants against a UnifiedGraph.
//! Each InvariantSpec is checked deterministically.
//! Reports which invariants pass/fail.

use vantage_types::invariant_spec::*;
use vantage_types::uir::UnifiedGraph;
use vantage_types::intent::IntentInvariant;
use vantage_types::symbol::SymbolKind;

/// Validate all invariants against a UnifiedGraph.
/// Returns a report with per-invariant pass/fail results.
pub fn validate_invariants(graph: &UnifiedGraph, specs: &[InvariantSpec]) -> InvariantReport {
    let mut results = Vec::with_capacity(specs.len());
    let mut passed = 0;
    let mut failed = 0;

    for spec in specs {
        let result = check_invariant(graph, spec);
        if result.passed { passed += 1; } else { failed += 1; }
        results.push(result);
    }

    InvariantReport {
        total: specs.len(),
        passed,
        failed,
        results,
    }
}

fn check_invariant(graph: &UnifiedGraph, spec: &InvariantSpec) -> InvariantResult {
    // Filter nodes by scope first
    let scope_filtered: Vec<_> = graph.nodes.iter().filter(|n| {
        match &spec.scope {
            InvariantScope::Global => true,
            InvariantScope::Language(lang) => n.language.as_str() == lang,
            InvariantScope::Module(prefix) => n.fq_name.starts_with(prefix),
        }
    }).collect();

    match &spec.rule {
        InvariantRule::NodeCountMin(min) => {
            let actual = scope_filtered.len();
            InvariantResult {
                spec_id: spec.id.clone(),
                passed: actual >= *min,
                message: if actual >= *min {
                    format!("OK: {} nodes (min {})", actual, min)
                } else {
                    format!("FAIL: {} nodes < min {}", actual, min)
                },
            }
        }

        InvariantRule::RequiredKind(kind) => {
            let has_kind = scope_filtered.iter().any(|n| n.kind == *kind);
            InvariantResult {
                spec_id: spec.id.clone(),
                passed: has_kind,
                message: if has_kind {
                    format!("OK: found {:?} node", kind)
                } else {
                    format!("FAIL: no {:?} node found", kind)
                },
            }
        }

        InvariantRule::RequiredIntent(intent) => {
            let has_intent = scope_filtered.iter().any(|n| {
                n.intent.as_ref().map_or(false, |i| i.invariant == *intent)
            });
            InvariantResult {
                spec_id: spec.id.clone(),
                passed: has_intent,
                message: if has_intent {
                    format!("OK: found {:?} intent", intent)
                } else {
                    format!("FAIL: no {:?} intent found", intent)
                },
            }
        }

        InvariantRule::CrossLanguageParity(symbol_name) => {
            let languages: std::collections::HashSet<_> = scope_filtered.iter()
                .filter(|n| n.fq_name.contains(symbol_name))
                .map(|n| n.language)
                .collect();
            let multiple = languages.len() > 1;
            InvariantResult {
                spec_id: spec.id.clone(),
                passed: multiple,
                message: if multiple {
                    format!("OK: '{}' found in {} languages", symbol_name, languages.len())
                } else {
                    format!("FAIL: '{}' found in {:?} only", symbol_name, languages.iter().next())
                },
            }
        }

        InvariantRule::HashStability => {
            // HashStability is validated at the verify command level
            // by re-running and comparing. At the single-snapshot level,
            // we verify that every node has a non-empty hash.
            let all_hashed = scope_filtered.iter().all(|n| !n.structural_hash.is_empty());
            InvariantResult {
                spec_id: spec.id.clone(),
                passed: all_hashed,
                message: if all_hashed {
                    "OK: all nodes have structural hash".to_string()
                } else {
                    "FAIL: some nodes missing structural hash".to_string()
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vantage_types::uir::{Language, UnifiedNode, Visibility};
    use vantage_types::SymbolId;

    fn make_test_graph() -> UnifiedGraph {
        UnifiedGraph {
            nodes: vec![
                UnifiedNode {
                    id: SymbolId::new("crate::Arena"),
                    fq_name: "crate::Arena".to_string(),
                    language: Language::Rust,
                    kind: SymbolKind::Struct,
                    visibility: Visibility::Public,
                    dependencies: vec![],
                    file: "arena.rs".to_string(),
                    line: 1,
                    structural_hash: "abc".to_string(),
                    normalized_hash: "abc".to_string(),
                    intent: None,
                },
                UnifiedNode {
                    id: SymbolId::new("crate::Scheduler"),
                    fq_name: "crate::Scheduler".to_string(),
                    language: Language::Rust,
                    kind: SymbolKind::Struct,
                    visibility: Visibility::Public,
                    dependencies: vec![],
                    file: "scheduler.rs".to_string(),
                    line: 10,
                    structural_hash: "def".to_string(),
                    normalized_hash: "def".to_string(),
                    intent: Some(vantage_types::intent::IntentOverlay {
                        invariant: IntentInvariant::DeterministicOrdering,
                        reason: "test".to_string(),
                        constraints: vec![],
                        owner: None,
                        metadata: std::collections::HashMap::new(),
                    }),
                },
            ],
            source_language: Language::Rust,
        }
    }

    #[test]
    fn test_node_count_min_passes() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "min-nodes".to_string(),
            description: "".to_string(),
            rule: InvariantRule::NodeCountMin(2),
            scope: InvariantScope::Global,
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.passed == 1);
        assert!(report.results[0].passed);
    }

    #[test]
    fn test_node_count_min_fails() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "min-nodes".to_string(),
            description: "".to_string(),
            rule: InvariantRule::NodeCountMin(99),
            scope: InvariantScope::Global,
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.failed == 1);
    }

    #[test]
    fn test_required_kind() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "has-struct".to_string(),
            description: "".to_string(),
            rule: InvariantRule::RequiredKind(SymbolKind::Struct),
            scope: InvariantScope::Global,
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.passed == 1);
    }

    #[test]
    fn test_required_intent() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "has-deterministic".to_string(),
            description: "".to_string(),
            rule: InvariantRule::RequiredIntent(IntentInvariant::DeterministicOrdering),
            scope: InvariantScope::Global,
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.passed == 1);
    }

    #[test]
    fn test_missing_intent_fails() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "has-append-only".to_string(),
            description: "".to_string(),
            rule: InvariantRule::RequiredIntent(IntentInvariant::AppendOnly),
            scope: InvariantScope::Global,
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.failed == 1);
    }

    #[test]
    fn test_scope_language() {
        let g = make_test_graph();
        let specs = vec![InvariantSpec {
            id: "rust-node-count".to_string(),
            description: "".to_string(),
            rule: InvariantRule::NodeCountMin(2),
            scope: InvariantScope::Language("rust".to_string()),
        }];
        let report = validate_invariants(&g, &specs);
        assert!(report.passed == 1);
    }
}
