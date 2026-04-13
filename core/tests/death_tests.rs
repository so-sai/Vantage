//! # Vantage Core Death Tests (Phase B & C Invariants)
//!
//! Strictly enforces O(depth) dirty propagation, short-circuit constraints,
//! and Bipartite Identity stability for million-line AST mutations.

use vantage_types::NodeId;
use vantage_types::SymbolId;
use vantage_types::graph::SymbolState;
use vantage_types::arena::NodeArena;
use vantage_types::dirty_propagation::DirtyPropagator;
use vantage_types::telemetry::PerfMetrics;
use vantage_types::caf::{CafNode, CafHash};
use vantage_types::semantic::Commutativity;
use vantage_types::node_stamp::NodeStamp;
use vantage_core::SymbolDependencyGraph;
use vantage_core::DepNode;

fn mock_caf_node(parent_id: Option<NodeId>) -> CafNode {
    CafNode {
        kind: "Mock".to_string(),
        semantic_id: None,
        children: vec![],
        commutativity: Commutativity::OrderSensitive,
        byte_start: 0,
        byte_end: 0,
        parent_id,
    }
}

fn mock_stamp() -> NodeStamp {
    NodeStamp::new(CafHash::sha256(b"mock"), 1)
}

// ---------------------------------------------------------
// 1. PHASE B INVARIANT: O(Depth) & Short-Circuit Walk
// ---------------------------------------------------------
#[test]
fn test_invariant_o_depth_short_circuit() {
    // Setup: Simulate a 3-level AST branch: Root -> Parent -> Leaf
    let mut arena = NodeArena::new();
    let root_id = NodeId(1);
    let parent_id = NodeId(2);
    let leaf_id = NodeId(3);

    // Simulate inserting nodes into Arena (Already parsed)
    arena.insert(root_id, mock_caf_node(None), mock_stamp());
    arena.insert(parent_id, mock_caf_node(Some(root_id)), mock_stamp());
    arena.insert(leaf_id, mock_caf_node(Some(parent_id)), mock_stamp());

    let mut metrics = PerfMetrics::new();

    // Action 1: First time AI edits the Leaf
    let depth_walked = {
        let mut propagator = DirtyPropagator::new(&mut arena);
        propagator.mark_dirty(leaf_id, &mut metrics)
    };
    
    // Assert 1: Must climb exactly 3 levels (Leaf -> Parent -> Root)
    assert_eq!(depth_walked, 3, "FATAL: Dirty propagation failed O(depth) constraint");
    assert!(arena.get(&root_id).unwrap().stamp.dirty);

    // Action 2: AI edits the Leaf AGAIN
    // Core logic: Since Leaf itself is already dirty, the algorithm must short-circuit IMMEDIATELY.
    let depth_walked_again = {
        let mut propagator = DirtyPropagator::new(&mut arena);
        propagator.mark_dirty(leaf_id, &mut metrics)
    };

    // Assert 2: Touches Leaf, sees dirty -> STOPS before even climbing. Latency O(1). Returned depth is 0!
    assert_eq!(depth_walked_again, 0, "FATAL: Short-circuit failed. Engine is wasting CPU cycles!");

    // Action 3: AI edits a Sibling
    let sibling_id = NodeId(4);
    arena.insert(sibling_id, mock_caf_node(Some(parent_id)), mock_stamp());
    
    let depth_walked_sibling = {
        let mut propagator = DirtyPropagator::new(&mut arena);
        propagator.mark_dirty(sibling_id, &mut metrics)
    };

    // Assert 3: Marks sibling (1), climbs to parent, sees parent is dirty -> STOPS!
    assert_eq!(depth_walked_sibling, 1, "FATAL: Parent short-circuit failed. Engine climbed too high!");
}

// ---------------------------------------------------------
// 2. PHASE C INVARIANT: Symbol State Machine (Tombstone GC)
// ---------------------------------------------------------
#[test]
fn test_invariant_symbol_tombstone_graceful_eviction() {
    let mut graph = SymbolDependencyGraph::new();
    let sym_a = SymbolId::new("crate::module::AgentLogic");
    
    // Simulate Generation 1: File reading
    graph.current_generation = 1;
    let mut node = DepNode::new(sym_a.clone(), "test.rs", 0, graph.current_generation);
    node.state = SymbolState::Validated;
    graph.nodes.insert(sym_a.clone(), node);

    // Simulate Generation 2: AI decides to delete this function
    graph.current_generation = 2;
    graph.mark_tombstones(); // Automatically marks nodes older than current_generation as Tombstoned
    
    // Assert 3: Deleted symbol MUST NOT disappear immediately (Prevents dangling edges)
    assert!(graph.nodes.contains_key(&sym_a), "FATAL: Symbol deleted too early, causes ghost edges");
    assert_eq!(graph.nodes.get(&sym_a).unwrap().state, SymbolState::Tombstoned);
    assert_eq!(graph.nodes.get(&sym_a).unwrap().generation, 2, "Tombstones must have their generation bumped for grace period");

    // Simulate Generation 3: Garbage Collection
    graph.current_generation = 3;
    graph.gc(); // Should evict generating < 3

    // Assert 4: GC must cleanly remove it on the next generation
    assert!(!graph.nodes.contains_key(&sym_a), "FATAL: Memory Leak! Tombstoned node not evicted.");
}
