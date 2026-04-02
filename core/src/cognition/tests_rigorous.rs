#[cfg(test)]
mod tests {
    use crate::cognition::*;
    use crate::parser::EpistemicParser;
    use proptest::proptest;

    fn setup_rust_parser() -> EpistemicParser {
        EpistemicParser::new_rust_parser().unwrap()
    }

    fn setup_python_parser() -> EpistemicParser {
        EpistemicParser::new_python_parser().unwrap()
    }

    #[test]
    fn test_orphan_tag_rejection() {
        let mut parser = setup_rust_parser();

        // Case 1: Comment đơn thuần, không có target
        let source = "// @epistemic:orphan-uuid\n\n   ";

        let signals = parser.parse_signals(source, "test.rs");
        assert!(
            signals.is_empty(),
            "Orphan tag không được phép tạo ra signal"
        );
    }

    #[test]
    fn test_simple_function_signal() {
        let mut parser = setup_rust_parser();

        let source = "// @epistemic:func-uuid\nfn target_function() {}";

        let signals = parser.parse_signals(source, "test.rs");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].uuid, "func-uuid");
        assert_eq!(signals[0].symbol_id, "target_function");
    }

    #[test]
    fn test_python_function_signal() {
        let mut parser = setup_python_parser();

        let source = "# @epistemic:py-func-uuid\ndef python_func():\n    pass";

        let signals = parser.parse_signals(source, "test.rs");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].uuid, "py-func-uuid");
        assert_eq!(signals[0].symbol_id, "python_func");
        assert_eq!(signals[0].language, "python");
    }

    #[test]
    fn test_struct_signal() {
        let mut parser = setup_rust_parser();

        let source = "// @epistemic:struct-uuid\nstruct TestStruct {}";

        let signals = parser.parse_signals(source, "test.rs");
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].uuid, "struct-uuid");
    }

    #[test]
    fn test_void_traversal() {
        let mut parser = setup_rust_parser();

        let source = r#"
            // @epistemic:uuid-123
            
            // Comment rác
            /* Comment block rác */
            
            fn actual_target() {
                println!("code here");
            }
        "#;

        let signals = parser.parse_signals(source, "test.rs");
        assert_eq!(signals.len(), 1, "Phải xuyên qua VOID để tìm SOLID");
        assert_eq!(signals[0].uuid, "uuid-123");
    }

    #[test]
    fn test_drift_detection() {
        let mut parser = setup_rust_parser();

        let source_v1 = "// @epistemic:drift-test\nfn target() { old(); }";
        let source_v2 = "// @epistemic:drift-test\nfn target() { new(); }";

        let signal_v1 = &parser.parse_signals(source_v1, "test.rs")[0];
        let signal_v2 = &parser.parse_signals(source_v2, "test.rs")[0];

        assert_ne!(
            signal_v1.structural_hash, signal_v2.structural_hash,
            "Structural Hash phải thay đổi khi nội dung SOLID thay đổi"
        );
    }

    #[test]
    fn test_semantic_vs_structural_hash() {
        let mut parser = setup_rust_parser();

        let source_v1 = "// @epistemic:hash-test\nfn target() { logic(); }";
        let source_v2 = "// @epistemic:hash-test\nfn  target()  {   logic();   }";

        let signal_v1 = &parser.parse_signals(source_v1, "test.rs")[0];
        let signal_v2 = &parser.parse_signals(source_v2, "test.rs")[0];

        // Structural hash should change due to whitespace
        assert_ne!(signal_v1.structural_hash, signal_v2.structural_hash);

        // Semantic hash should remain same (whitespace stripped)
        assert_eq!(
            signal_v1.semantic_hash, signal_v2.semantic_hash,
            "Semantic hash phải nhất quán dù có thay đổi whitespace"
        );
    }

    // Property test với realistic junk patterns
    proptest! {
        #[test]
        fn test_with_realistic_junk(junk in r#"[ \t]*//[^\n]*\n?"#) {
            let mut parser = setup_rust_parser();

            let source = format!(
                "// @epistemic:FUZZ-ID\n{}\nfn target() {{}}",
                junk
            );

            let signals = parser.parse_signals(&source, "test.rs");

            if !signals.is_empty() {
                assert_eq!(signals[0].uuid, "FUZZ-ID");
            }
        }
    }

    #[test]
    fn test_comment_only_file() {
        let mut parser = setup_rust_parser();

        let source = r#"
            // @epistemic:comment-only
            // Another comment
            /* Yet another */
        "#;

        let signals = parser.parse_signals(source, "test.rs");
        assert!(
            signals.is_empty(),
            "File chỉ có comment không được tạo signal"
        );
    }

    #[test]
    fn test_unicode_junk_resilience() {
        let mut parser = setup_rust_parser();

        // Testing resilience to NBSP (\u{a0}) + complex junk (Fuzzer match)
        let source = "// @epistemic:unicode-test\n\u{a0}//{a{A:\nfn target() {}";

        let signals = parser.parse_signals(source, "test.rs");
        assert_eq!(signals.len(), 1, "Parser phải vượt qua NBSP + junk");
        assert_eq!(signals[0].uuid, "unicode-test");
    }

    #[test]
    fn test_empty_input() {
        let mut parser = setup_rust_parser();

        let signals = parser.parse_signals("", "test.rs");
        assert!(signals.is_empty(), "Input rỗng không được tạo signal");
    }

    // ============================================================
    // DETERMINISM TESTS (v1.2.4 Trust Hardening)
    // ============================================================

    #[test]
    fn test_determinism_same_source_same_hash() {
        let mut parser = setup_rust_parser();
        let source = r#"
            // @epistemic:det-test-1
            fn alpha() {
                println!("hello");
            }

            // @epistemic:det-test-2
            struct Beta {
                field: u32,
            }
        "#;

        // Parse 100 times, assert all hashes match
        let mut runs: Vec<Vec<CognitiveSignal>> = Vec::new();
        for _ in 0..100 {
            let signals = parser.parse_signals(source, "test.rs");
            runs.push(signals);
        }

        let reference = &runs[0];
        assert_eq!(reference.len(), 2, "Should detect 2 signals");

        for (i, run) in runs.iter().enumerate().skip(1) {
            assert_eq!(
                reference.len(),
                run.len(),
                "Run {} produced different signal count",
                i
            );
            for (ref_sig, run_sig) in reference.iter().zip(run.iter()) {
                assert_eq!(
                    ref_sig.structural_hash, run_sig.structural_hash,
                    "Run {} structural_hash mismatch for {}",
                    i, ref_sig.symbol_id
                );
                assert_eq!(
                    ref_sig.semantic_hash, run_sig.semantic_hash,
                    "Run {} semantic_hash mismatch for {}",
                    i, ref_sig.symbol_id
                );
                assert_eq!(
                    ref_sig.normalized_hash, run_sig.normalized_hash,
                    "Run {} normalized_hash mismatch for {}",
                    i, ref_sig.symbol_id
                );
            }
        }
    }

    #[test]
    fn test_determinism_line_ending_normalization() {
        let mut parser = setup_rust_parser();

        // LF source (Unix)
        let source_lf = "// @epistemic:crlf-test\nfn target() {\n    let x = 1;\n}\n";

        // CRLF source (Windows)
        let source_crlf = "// @epistemic:crlf-test\r\nfn target() {\r\n    let x = 1;\r\n}\r\n";

        let signals_lf = parser.parse_signals(source_lf, "test.rs");
        let signals_crlf = parser.parse_signals(source_crlf, "test.rs");

        assert_eq!(signals_lf.len(), 1, "LF source should produce 1 signal");
        assert_eq!(signals_crlf.len(), 1, "CRLF source should produce 1 signal");
        assert_eq!(
            signals_lf[0].structural_hash, signals_crlf[0].structural_hash,
            "CRLF vs LF must produce identical structural_hash"
        );
        assert_eq!(
            signals_lf[0].semantic_hash, signals_crlf[0].semantic_hash,
            "CRLF vs LF must produce identical semantic_hash"
        );
    }

    #[test]
    fn test_determinism_signal_ordering() {
        let mut parser = setup_rust_parser();
        let source = r#"
            // @epistemic:order-z
            fn zebra() {}

            // @epistemic:order-a
            fn alpha() {}

            // @epistemic:order-m
            fn middle() {}
        "#;

        let mut all_orderings = Vec::new();
        for _ in 0..20 {
            let signals = parser.parse_signals(source, "test.rs");
            let order: Vec<String> = signals.iter().map(|s| s.symbol_id.clone()).collect();
            all_orderings.push(order);
        }

        let reference = &all_orderings[0];
        for (i, order) in all_orderings.iter().enumerate().skip(1) {
            assert_eq!(
                reference, order,
                "Run {} produced different signal ordering",
                i
            );
        }

        // Should be sorted by byte_start (zebra appears first in source)
        assert_eq!(
            reference[0], "zebra",
            "Signals should be sorted by byte position"
        );
    }

    #[test]
    fn test_determinism_graph_edge_ordering() {
        let mut parser = setup_rust_parser();
        let source = r#"
            // @epistemic:graph-test
            fn caller() {
                zebra();
                alpha();
                middle();
            }

            fn zebra() {}
            fn alpha() {}
            fn middle() {}
        "#;

        let mut all_edge_orders = Vec::new();
        for _ in 0..20 {
            let (_signals, graph) = parser.parse_with_graph(source, "test.rs");
            let edge_order: Vec<String> = graph
                .sorted_edges()
                .into_iter()
                .map(|e| format!("{}->{}", e.from, e.to))
                .collect();
            all_edge_orders.push(edge_order);
        }

        let reference = &all_edge_orders[0];
        for (i, order) in all_edge_orders.iter().enumerate().skip(1) {
            assert_eq!(
                reference, order,
                "Run {} produced different edge ordering",
                i
            );
        }
    }

    #[test]
    fn test_rename_invariant() {
        let mut parser = setup_rust_parser();

        let source_original = r#"
            // @epistemic:rename-test
            fn calculate_total(price: u32, quantity: u32) -> u32 {
                price * quantity
            }
        "#;

        let source_renamed = r#"
            // @epistemic:rename-test
            fn calc(p: u32, q: u32) -> u32 {
                p * q
            }
        "#;

        let sig_original = &parser.parse_signals(source_original, "test.rs")[0];
        let sig_renamed = &parser.parse_signals(source_renamed, "test.rs")[0];

        assert_eq!(
            sig_original.normalized_hash, sig_renamed.normalized_hash,
            "Renaming variables and functions must NOT change normalized_hash"
        );
        assert_ne!(
            sig_original.structural_hash, sig_renamed.structural_hash,
            "Renaming MUST change structural_hash (detects any physical change)"
        );
        assert_ne!(
            sig_original.semantic_hash, sig_renamed.semantic_hash,
            "Renaming MUST change semantic_hash (identifiers are semantic)"
        );
    }

    #[test]
    fn test_drift_classification_all_variants() {
        use crate::DriftReport;
        use crate::DriftStatus;

        // Baseline: 3 signals
        let mut parser = setup_rust_parser();
        let baseline_source = r#"
            // @epistemic:drift-a
            fn alpha() { let x = 1; }

            // @epistemic:drift-b
            fn beta() { let x = 2; }

            // @epistemic:drift-c
            fn gamma() { let x = 3; }
        "#;
        let baseline = parser.parse_signals(baseline_source, "test.rs");
        assert_eq!(baseline.len(), 3, "Baseline must have 3 signals");

        // Current: alpha unchanged, beta struct-changed (whitespace), gamma semantically changed, delta added
        let current_source = r#"
            // @epistemic:drift-a
            fn alpha() { let x = 1; }

            // @epistemic:drift-b
            fn  beta()  {  let  x  =  2;  }

            // @epistemic:drift-c
            fn gamma() { if true { let x = 3; } }

            // @epistemic:drift-delta
            fn delta() {}
        "#;
        let current = parser.parse_signals(current_source, "test.rs");

        let report = DriftReport::compare(&baseline, &current);

        let status_map: std::collections::HashMap<&str, DriftStatus> = report
            .items
            .iter()
            .map(|item| (item.symbol_id.as_str(), item.status))
            .collect();

        assert_eq!(
            status_map.get("alpha"),
            Some(&DriftStatus::Unchanged),
            "alpha should be unchanged"
        );
        assert_eq!(
            status_map.get("beta"),
            Some(&DriftStatus::StructuralChange),
            "beta should be structural change (whitespace)"
        );
        assert_eq!(
            status_map.get("gamma"),
            Some(&DriftStatus::SemanticChange),
            "gamma should be semantic change (logic change)"
        );
        assert_eq!(
            status_map.get("delta"),
            Some(&DriftStatus::Added),
            "delta should be added"
        );
        assert_eq!(
            status_map.get("gamma"),
            Some(&DriftStatus::SemanticChange),
            "gamma should still be present (not removed)"
        );

        // Verify removed count: none removed since all baseline symbols still in current
        assert_eq!(report.removed, 0, "No symbols should be removed");
        assert_eq!(report.added, 1, "delta should be the only added symbol");
    }
}
