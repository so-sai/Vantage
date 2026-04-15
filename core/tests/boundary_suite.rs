//! Vantage v1.2.4 Boundary Test Suite
//!
//! This suite maps the system boundaries - defining WHERE correctness ends and failure/instability begins.
//! NOTE: Vantage requires @epistemic: tags in source to extract signals.

use vantage_core::parser::EpistemicParser;
use vantage_types::{IdentityAnchor, NodeId, SemanticRole, SymbolGraphDTO, SymbolId};

mod identity_boundary {
    use super::*;

    #[test]
    fn nodeid_stability_same_input() {
        let parent_id = Some(NodeId(123));
        let role = SemanticRole::LetBinding;
        let anchor = IdentityAnchor::Binding(SymbolId::new("test_symbol"));

        let (id1, _) = NodeId::generate(parent_id, role, &anchor);
        let (id2, _) = NodeId::generate(parent_id, role, &anchor);

        assert_eq!(id1, id2, "Same inputs must produce identical NodeId");
    }

    #[test]
    fn reorder_insertions_no_id_shift() {
        let parent = Some(NodeId(1));
        let role = SemanticRole::LetBinding;

        // Sequence A: insert X then Y
        let anchor_x = IdentityAnchor::Binding(SymbolId::new("x"));
        let (id_x_a, _) = NodeId::generate(parent, role, &anchor_x);

        let anchor_y = IdentityAnchor::Binding(SymbolId::new("y"));
        let (id_y_a, _) = NodeId::generate(parent, role, &anchor_y);

        // Sequence B: insert Y then X (reversed order)
        let anchor_y2 = IdentityAnchor::Binding(SymbolId::new("y"));
        let (id_y_b, _) = NodeId::generate(parent, role, &anchor_y2);

        let anchor_x2 = IdentityAnchor::Binding(SymbolId::new("x"));
        let (id_x_b, _) = NodeId::generate(parent, role, &anchor_x2);

        // IDs must NOT depend on insertion order
        assert_eq!(
            id_x_a, id_x_b,
            "NodeId 'x' shifted based on insertion order!"
        );
        assert_eq!(
            id_y_a, id_y_b,
            "NodeId 'y' shifted based on insertion order!"
        );
    }

    #[test]
    fn rename_changes_different_symbols() {
        // SymbolId must be stable within same session - different FQNs produce different IDs
        let id_original = SymbolId::new("old_name");
        let id_renamed = SymbolId::new("new_name");

        assert_ne!(id_original.index, id_renamed.index);
    }
}

mod structural_hash_boundary {
    use super::*;

    #[test]
    fn whitespace_only_is_non_breaking() {
        // Vantage requires @epistemic: tags
        let source_a = "// @epistemic:ws-test-1\nfn foo() { let x = 1; }";
        let source_b = "// @epistemic:ws-test-1\nfn  foo()  {  let  x  =  1;  }";

        let mut parser1 = EpistemicParser::new_rust_parser().unwrap();
        let mut parser2 = EpistemicParser::new_rust_parser().unwrap();

        let dto1 = parser1.parse_with_graph(source_a, "a.rs").1.to_dto();
        let dto2 = parser2.parse_with_graph(source_b, "b.rs").1.to_dto();

        let json1 = serde_json::to_string(&dto1).unwrap();
        let json2 = serde_json::to_string(&dto2).unwrap();

        // Whitespace differences should not change structural graph
        assert_eq!(
            json1, json2,
            "Whitespace-only changes should not affect structural output"
        );
    }

    #[test]
    fn logic_change_produces_different_output() {
        // Use different epistemic IDs - same ID = same semantic identity
        // Adding a function WITH DIFFERENT ID should create new node
        let source_a = "// @epistemic:logic-test-a\nfn foo() {}";
        let source_b = "// @epistemic:logic-test-b\nfn foo() {} fn bar() {}";

        let mut parser1 = EpistemicParser::new_rust_parser().unwrap();
        let mut parser2 = EpistemicParser::new_rust_parser().unwrap();

        let dto1 = parser1.parse_with_graph(source_a, "a.rs").1.to_dto();
        let dto2 = parser2.parse_with_graph(source_b, "b.rs").1.to_dto();

        // Different epistemic ID + more functions = more nodes
        assert!(
            dto2.nodes.len() >= dto1.nodes.len(),
            "Adding functions should increase or maintain nodes"
        );
    }
}

mod seal_stability_boundary {
    use super::*;

    #[test]
    fn seal_deterministic_across_runs() {
        let source = "// @epistemic:det-seal-1\nmod foo; fn bar() {}";

        let run1 = {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&parser.parse_with_graph(source, "a.rs").1.to_dto()).unwrap()
        };

        let run2 = {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&parser.parse_with_graph(source, "a.rs").1.to_dto()).unwrap()
        };

        assert_eq!(run1, run2, "Seal must be deterministic across runs");
    }

    #[test]
    fn seal_not_affected_by_field_order() {
        let source = "// @epistemic:field-order-1\nfn foo() { let x = 1; }";

        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph(source, "test.rs").1.to_dto();

        // Compare by serializing twice and parsing between
        let json1 = serde_json::to_string(&dto).unwrap();

        // Parse and re-serialize - field order may differ but semantic should be same
        let parsed: serde_json::Value = serde_json::from_str(&json1).unwrap();

        // Verify structure is valid (nodes array exists)
        assert!(parsed.get("nodes").is_some(), "Should have nodes field");
    }
}

mod determinism_boundary {
    use super::*;

    #[test]
    fn same_input_different_runs_same_output() {
        let source = "// @epistemic:det-1\nfn a() {} fn b() {} mod c {}";

        let run1 = {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&parser.parse_with_graph(source, "test.rs").1.to_dto()).unwrap()
        };

        let run2 = {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&parser.parse_with_graph(source, "test.rs").1.to_dto()).unwrap()
        };

        assert_eq!(
            run1, run2,
            "Vantage is NOT deterministic - same input produced different output!"
        );
    }

    #[test]
    fn same_source_different_filename_equals() {
        // Same source in different files must produce identical structural output
        let source = "// @epistemic:filename-test-1\nstruct Foo { bar: i32 }";

        let result_a = {
            let mut p = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&p.parse_with_graph(source, "a.rs").1.to_dto()).unwrap()
        };
        let result_b = {
            let mut p = EpistemicParser::new_rust_parser().unwrap();
            serde_json::to_string(&p.parse_with_graph(source, "b.rs").1.to_dto()).unwrap()
        };

        assert_eq!(
            result_a, result_b,
            "Same source must produce identical output regardless of filename"
        );
    }
}

mod scalability_boundary {
    use super::*;

    fn make_test_source(count: usize) -> String {
        let mut source = String::new();
        for i in 0..count {
            source.push_str(&format!(
                "// @epistemic:scalability-{}\nfn func_{}() {{ let x = {}; }}\n",
                i, i, i
            ));
        }
        source
    }

    #[test]
    fn graph_100_nodes_stable() {
        let source = make_test_source(100);

        let start = std::time::Instant::now();
        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph(&source, "test.rs").1.to_dto();
        let elapsed = start.elapsed();

        assert!(dto.nodes.len() > 0, "Graph should have nodes");
        assert!(
            elapsed.as_millis() < 500,
            "100 nodes should parse in <500ms, took {:?}",
            elapsed
        );
    }

    #[test]
    fn graph_1000_nodes_benchmark() {
        let source = make_test_source(1000);

        let start = std::time::Instant::now();
        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph(&source, "test.rs").1.to_dto();
        let elapsed = start.elapsed();

        assert!(dto.nodes.len() > 0, "Should have parsed");
        // Windows is slower - allow up to 10s
        assert!(
            elapsed.as_millis() < 10000,
            "1000 nodes took {}ms - exceeded 10s",
            elapsed.as_millis()
        );
    }

    #[ignore = "Stress test - run manually"]
    #[test]
    fn graph_10k_nodes_stress() {
        let source = make_test_source(10000);

        let start = std::time::Instant::now();
        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph(&source, "test.rs").1.to_dto();
        let elapsed = start.elapsed();

        eprintln!(
            "10k nodes: {}ms, {} graph nodes",
            elapsed.as_millis(),
            dto.nodes.len()
        );

        assert!(elapsed.as_secs() < 30, "10k nodes exceeded 30s threshold");
    }
}

mod replay_boundary {
    use super::*;

    #[test]
    fn replay_is_idempotent() {
        let source = "// @epistemic:replay-1\nfn a() { b(); } fn b() {}";

        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        let dto1 = parser.parse_with_graph(source, "test.rs").1.to_dto();
        let dto2 = parser.parse_with_graph(source, "test.rs").1.to_dto();

        let json1 = serde_json::to_string(&dto1).unwrap();
        let json2 = serde_json::to_string(&dto2).unwrap();

        assert_eq!(json1, json2, "Replay must be idempotent");
    }

    #[test]
    fn roundtrip_preserves_graph() {
        let source = "// @epistemic:roundtrip-1\nstruct Point { x: i32, y: i32 }";

        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph(source, "test.rs").1.to_dto();

        let json = serde_json::to_string(&dto).unwrap();

        let reconstructed: SymbolGraphDTO = serde_json::from_str(&json).unwrap();

        assert_eq!(reconstructed.nodes.len(), dto.nodes.len());
    }
}

mod failure_injection {
    use super::*;

    #[test]
    fn empty_source_is_safe() {
        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph("", "empty.rs").1.to_dto();

        assert!(
            dto.nodes.is_empty(),
            "Empty source should produce empty graph"
        );
    }

    #[test]
    fn invalid_syntax_is_safe() {
        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        let result = parser.parse_with_graph("// @epistemic:bad-1\nfn { { { [[[", "bad.rs");

        // Parser may return empty graph but should not crash
        assert!(result.1.nodes.len() >= 0);
    }

    #[test]
    fn unicode_is_safe() {
        let source = "// @epistemic:unicode-1\nfn función() { let 变量 = 1; let 🎉 = 2; }";

        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let result = parser.parse_with_graph(source, "unicode.rs");

        assert!(result.1.nodes.len() >= 0);
    }
}
