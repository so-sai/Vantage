use vantage_types::SymbolId;
use vantage_types::graph::{SymbolState, DependencyKind};
use vantage_core::parser::EpistemicParser;
use vantage_core::graph::SymbolDependencyGraph;
use std::sync::Arc;

// ============================================================================
// LAYER 1: INTERNER STABILITY (IDENTITY PHYSICS)
// ============================================================================

#[test]
fn test_identity_uniqueness_pointer_eq() {
    let fqn = "vantage::core::parser::EpistemicParser";
    let id1 = SymbolId::new(fqn);
    let id2 = SymbolId::new(fqn);

    // RULE: Arc::ptr_eq must be true for identical FQNs
    assert!(Arc::ptr_eq(&id1.to_string(), &id2.to_string()), "Identity split! Pointer equality failed.");
    assert!(id1.identity_eq(&id2), "Forensic identity_eq failed.");
}

#[test]
fn test_interner_collision_resistance_stress() {
    // Stress test: 10,000 runs to ensure reuse stability
    let key = "stress_symbol";
    let first = SymbolId::new(key);
    for _ in 0..10000 {
        let current = SymbolId::new(key);
        assert!(Arc::ptr_eq(&first.to_string(), &current.to_string()), "Collision/Fragmentation in interner!");
    }

    // Negative test: Different inputs must never collide
    let id_a = SymbolId::new("symbol_a");
    let id_b = SymbolId::new("symbol_b");
    assert!(!Arc::ptr_eq(&id_a.to_string(), &id_b.to_string()), "False positive identity collision!");
}

#[test]
fn test_canonicalite_idempotency_closure() {
    let raw = "::::crate:::foo::";
    let id1 = SymbolId::new(raw);
    let id2 = SymbolId::new(&id1.to_string());
    let id3 = SymbolId::new(&id2.to_string());

    // RULE: normalize(normalize(x)) == normalize(x)
    assert!(Arc::ptr_eq(&id1.to_string(), &id2.to_string()), "Canonicalization is NOT idempotent (Run 2)");
    assert!(Arc::ptr_eq(&id2.to_string(), &id3.to_string()), "Canonicalization is NOT idempotent (Run 3)");
}

// ============================================================================
// LAYER 2: IDENTITY BOUNDARY & LEAKAGE
// ============================================================================

#[test]
fn test_identity_boundary_integrity() {
    let source = "fn target() {}";
    let mut parser = EpistemicParser::new_rust_parser().unwrap();
    let (_, graph) = parser.parse_with_graph(source, "leak_test.rs");
    
    // Ensure all nodes in graph are interned by matching pointers
    // with a second intern call for the same FQN string.
    for node in graph.nodes.values() {
        let fqn = node.symbol.to_string().as_ref();
        let verification_ptr = vantage_types::registry().intern(fqn);
        assert!(Arc::ptr_eq(&node.symbol.to_string(), &verification_ptr.to_string()), 
            "Identity Leakage! Node for {} contains an uninterned/raw Arc.", fqn);
    }
}

// ============================================================================
// LAYER 3: GRAPH EVOLUTION & MUTATION
// ============================================================================

#[test]
fn test_graph_mutation_determinism_invariant() {
    let mut g1 = SymbolDependencyGraph::new();
    let mut g2 = SymbolDependencyGraph::new();
    
    let a = SymbolId::new("A");
    let b = SymbolId::new("B");
    
    // Mutation Sequence
    let ops = |g: &mut SymbolDependencyGraph| {
        g.add_node(a.clone(), "f1.rs", 10);
        g.add_node(b.clone(), "f2.rs", 20);
        g.add_edge(&a, &b, DependencyKind::CallEdge);
        g.bump_generation();
    };

    ops(&mut g1);
    ops(&mut g2);

    let dto1 = g1.to_dto();
    let dto2 = g2.to_dto();

    // Verify same mutation sequence -> same DTO state
    assert_eq!(dto1.nodes.len(), dto2.nodes.len());
    
    for i in 0..dto1.nodes.len() {
        assert_eq!(dto1.nodes[i].symbol, dto2.nodes[i].symbol);
        assert_eq!(dto1.nodes[i].dependencies.len(), dto2.nodes[i].dependencies.len());
    }
}

#[test]
fn test_symbol_lifecycle_state_transition() {
    let mut g = SymbolDependencyGraph::new();
    let id = SymbolId::new("lifecycle_test");
    
    // Pass 1: Discover
    g.add_node(id.clone(), "file.rs", 1);
    {
        let node = g.nodes.get(&id).unwrap();
        assert_eq!(node.state, SymbolState::Discovered);
    }

    // Pass 2: Node disappears from discovery
    g.bump_generation(); // current_gen = 1. Node in graph remains at gen 0
    g.mark_tombstones(); // Marks gen 0 nodes as Tombstoned and bumps them to gen 1
    {
        let node = g.nodes.get(&id).unwrap();
        assert_eq!(node.state, SymbolState::Tombstoned);
        assert_eq!(node.generation, 1);
    }

    // Pass 3: GC run
    g.gc();
    assert!(g.nodes.contains_key(&id), "Tombstoned node must stay in graph for the remainder of its marking generation.");

    g.bump_generation(); // current_gen = 2
    g.gc();
    assert!(!g.nodes.contains_key(&id), "Tombstoned node should be evicted when a new generation starts without its presence.");
}
