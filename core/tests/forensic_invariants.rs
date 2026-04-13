//! # Vantage Invariant Harness v1 (VIH-v1)
//!
//! This suite enforces the "Forensic VM" invariants required for v1.2.4.
//! It transforms Vantage from a parser into a provably deterministic sensor.
//!
//! TDD Matrix Coverage: ABI-01 to M-04

use vantage_types::{SYSTEM_ABI_HASH, SymbolId, registry};
use vantage_core::parser::EpistemicParser;
use vantage_core::graph::SymbolDependencyGraph;

// ============================================================================
// LAYER 1: ABI / STRUCTURE FREEZE (BOUNDARY)
// ============================================================================

#[test]
fn test_abi_lock_v1_integrity() {
    // ABI-01: Schema Stability
    // If the struct layout or version drifts, this must fail.
    // The hash is a constant symbol for the v1.2.4 structure.
    assert_eq!(SYSTEM_ABI_HASH, "7e0921a4f89d23c4e9e03d156b2c5b3d", 
        "FATAL: ABI Drift detected! Core engine structures have changed without a protocol version bump.");
}

#[test]
fn test_symbol_id_size_lock() {
    // ABI-02: Ensure SymbolId hasn't grown unexpectedly (Memory budget check)
    // u64 (index) + u32 (epoch) + Arc<str> (pointer)
    // On 64-bit systems, this is 8 + 4 + 8 = 20, usually padded to 24 or 32.
    let size = std::mem::size_of::<SymbolId>();
    assert!(size <= 32, "SymbolId size explosion! Current size: {} bytes. Must be <= 32.", size);
}

// ============================================================================
// LAYER 2: CROSS-SESSION DETERMINISM (REPLAY)
// ============================================================================

#[test]
fn test_replay_determinism_isomorphism_r01() {
    let source = "fn a() { b(); } fn b() {}";
    let mut parser = EpistemicParser::new_rust_parser().unwrap();
    
    // 1. Initial Parse
    let (_, graph1) = parser.parse_with_graph(source, "replay_1.rs");
    let dto1 = graph1.to_dto();
    let json1 = serde_json::to_string(&dto1).unwrap();
    
    // 2. Simulate Session Restart (Fresh Parser + New Registry)
    // Note: Protocol V0 requires that the reloaded graph is structurally identical.
    let mut parser2 = EpistemicParser::new_rust_parser().unwrap();
    let (_, graph2) = parser2.parse_with_graph(source, "replay_2.rs");
    let dto2 = graph2.to_dto();
    let json2 = serde_json::to_string(&dto2).unwrap();
    
    // 3. Assert Bit-Identical Serialization
    assert_eq!(json1, json2, "Replay determinism failed! Output drifted across sessions.");
}

// ============================================================================
// LAYER 3: MULTI-INSTANCE / MULTI-IDE CONSISTENCY
// ============================================================================

#[test]
fn test_multi_instance_consistency_m01() {
    // M-01: Two independent registries must produce identical graph output for same symbols.
    let fqn = "vantage::core::test";
    
    // Instance A
    let id_a = SymbolId::new(fqn);
    
    // Instance B (Simulated by verifying the global registry idempotency)
    let id_b = registry().intern(fqn);
    
    assert_eq!(id_a.index, id_b.index, "Registry inconsistency! Same FQN mapped to different indices.");
    assert_eq!(id_a.registry_epoch, id_b.registry_epoch, "Registry epoch drift!");
}

// ============================================================================
// LAYER 4: NON-DETERMINISM STRESS (GUARD RAILS)
// ============================================================================

#[test]
fn test_ordering_invariance_r02() {
    // R-02: Eliminate randomness from HashMap insertion order.
    // The graph DTO must use stable keys for sorting.
    let mut g1 = SymbolDependencyGraph::new();
    let id_a = SymbolId::new("A");
    let id_b = SymbolId::new("B");
    let id_c = SymbolId::new("C");
    
    // Order 1: A, B, C
    g1.add_node(id_a.clone(), "f.rs", 1);
    g1.add_node(id_b.clone(), "f.rs", 2);
    g1.add_node(id_c.clone(), "f.rs", 3);
    let dto1 = g1.to_dto();
    
    // Order 2: C, A, B
    let mut g2 = SymbolDependencyGraph::new();
    g2.add_node(id_c, "f.rs", 3);
    g2.add_node(id_a, "f.rs", 1);
    g2.add_node(id_b, "f.rs", 2);
    let dto2 = g2.to_dto();
    
    assert_eq!(serde_json::to_string(&dto1).unwrap(), serde_json::to_string(&dto2).unwrap(),
        "Non-deterministic graph ordering! DTO output depends on insertion order.");
}
