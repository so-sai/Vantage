use vantage_types::edge::UnifiedEdgeKind;
use vantage_types::graph::DependencyKind;
use vantage_types::intent::IntentOverlay;
use vantage_types::uir::{Language, UnifiedEdge, UnifiedGraph, UnifiedNode, Visibility};
use crate::graph::SymbolDependencyGraph;
use crate::parser::intent_extractor::IntentBlock;
use crate::CognitiveSignal;

fn map_dependency_kind(kind: DependencyKind) -> UnifiedEdgeKind {
    match kind {
        DependencyKind::SignatureRef => UnifiedEdgeKind::References,
        DependencyKind::CallEdge => UnifiedEdgeKind::CallsUnresolved,
        DependencyKind::ModuleImport => UnifiedEdgeKind::Imports,
        DependencyKind::ReExport => UnifiedEdgeKind::ReExport,
    }
}

fn map_parser_language(lang: &crate::parser::Language) -> Language {
    match lang {
        crate::parser::Language::Rust => Language::Rust,
        crate::parser::Language::Python => Language::Python,
        crate::parser::Language::Ruby => Language::Ruby,
        crate::parser::Language::Javascript => Language::JavaScript,
        crate::parser::Language::Typescript => Language::TypeScript,
        crate::parser::Language::Tsx => Language::Tsx,
    }
}

fn match_intent(signal_line: u32, blocks: &[IntentBlock]) -> Option<IntentOverlay> {
    // Match: intent block's end_line == signal's start_line ±1 tolerance
    let sl = signal_line as isize;
    blocks.iter().find(|b| {
        let el = b.end_line as isize;
        (el - sl).abs() <= 1
    }).map(|b| b.overlay.clone())
}

pub fn normalize_to_unified(
    signals: &[CognitiveSignal],
    graph: &SymbolDependencyGraph,
    parser_lang: &crate::parser::Language,
    intent_blocks: &[IntentBlock],
) -> UnifiedGraph {
    let lang = map_parser_language(parser_lang);

    let nodes: Vec<UnifiedNode> = signals
        .iter()
        .map(|sig| {
            let deps = graph
                .nodes
                .get(&sig.symbol_id)
                .map(|node| {
                    let mut edges: Vec<UnifiedEdge> = node
                        .dependencies
                        .iter()
                        .map(|dep| UnifiedEdge {
                            target: dep.target.clone(),
                            kind: map_dependency_kind(dep.kind),
                        })
                        .collect();
                    edges.sort_by(|a, b| a.target.to_string().cmp(&b.target.to_string()));
                    edges
                })
                .unwrap_or_default();

            UnifiedNode {
                id: sig.symbol_id.clone(),
                fq_name: sig.symbol_id.to_string(),
                language: lang,
                kind: sig.symbol_kind.clone(),
                visibility: Visibility::Public,
                dependencies: deps,
                file: sig.location.file.clone(),
                line: sig.location.start_line,
                structural_hash: sig.structural_hash.clone(),
                normalized_hash: sig.normalized_hash.clone(),
                intent: match_intent(sig.location.start_line, intent_blocks),
            }
        })
        .collect();

    UnifiedGraph {
        nodes,
        source_language: lang,
    }
}
