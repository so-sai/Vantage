#[cfg(test)]
mod tests {
    use crate::cognition::*;
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

        let signals = parser.parse_signals(source);
        assert!(
            signals.is_empty(),
            "Orphan tag không được phép tạo ra signal"
        );
    }

    #[test]
    fn test_simple_function_signal() {
        let mut parser = setup_rust_parser();

        let source = "// @epistemic:func-uuid\nfn target_function() {}";

        let signals = parser.parse_signals(source);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].uuid, "func-uuid");
        assert_eq!(signals[0].symbol_id, "target_function");
    }

    #[test]
    fn test_python_function_signal() {
        let mut parser = setup_python_parser();

        let source = "# @epistemic:py-func-uuid\ndef python_func():\n    pass";

        let signals = parser.parse_signals(source);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].uuid, "py-func-uuid");
        assert_eq!(signals[0].symbol_id, "python_func");
        assert_eq!(signals[0].language, "python");
    }

    #[test]
    fn test_struct_signal() {
        let mut parser = setup_rust_parser();

        let source = "// @epistemic:struct-uuid\nstruct TestStruct {}";

        let signals = parser.parse_signals(source);
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

        let signals = parser.parse_signals(source);
        assert_eq!(signals.len(), 1, "Phải xuyên qua VOID để tìm SOLID");
        assert_eq!(signals[0].uuid, "uuid-123");
    }

    #[test]
    fn test_drift_detection() {
        let mut parser = setup_rust_parser();

        let source_v1 = "// @epistemic:drift-test\nfn target() { old(); }";
        let source_v2 = "// @epistemic:drift-test\nfn target() { new(); }";

        let signal_v1 = &parser.parse_signals(source_v1)[0];
        let signal_v2 = &parser.parse_signals(source_v2)[0];

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

        let signal_v1 = &parser.parse_signals(source_v1)[0];
        let signal_v2 = &parser.parse_signals(source_v2)[0];

        // Structural hash should change due to whitespace
        assert_ne!(signal_v1.structural_hash, signal_v2.structural_hash);
        
        // Semantic hash should remain same (whitespace stripped)
        assert_eq!(signal_v1.semantic_hash, signal_v2.semantic_hash, "Semantic hash phải nhất quán dù có thay đổi whitespace");
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

            let signals = parser.parse_signals(&source);

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

        let signals = parser.parse_signals(source);
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

        println!("TREE DEBUG: {}", parser.debug_tree(source));

        let signals = parser.parse_signals(source);
        assert_eq!(signals.len(), 1, "Parser phải vượt qua NBSP + junk");
        assert_eq!(signals[0].uuid, "unicode-test");
    }

    #[test]
    fn test_empty_input() {
        let mut parser = setup_rust_parser();

        let signals = parser.parse_signals("");
        assert!(signals.is_empty(), "Input rỗng không được tạo signal");
    }
}
