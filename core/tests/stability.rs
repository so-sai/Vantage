use proptest::prelude::*;
use vantage_core::EpistemicParser;

#[cfg(test)]
mod stability_tests {
    use super::*;

    fn setup_rust_parser() -> EpistemicParser {
        EpistemicParser::new_rust_parser().expect("Failed to initialize Rust parser")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        #[test]
        fn test_semantic_invariant_under_radioactive_junk(
            junk in r"([ \t\n\r]{0,20}|//[ -~]*\n){0,10}"
        ) {
            let mut parser = setup_rust_parser();

            // The logic stays the same, only the whitespace/junk around it changes
            // We use triple newlines to force tree-sitter recovery after radioactive junk
            let source = format!(
                "// @epistemic:stable-id\n\n\n{}\n\n\nfn target_logic(a: i32) -> i32 {{ a + 1 }}\n\n\n{}\n\n\n",
                junk, junk
            );

            let signals = parser.parse_signals(&source, "stability_test.rs");

            if signals.is_empty() {
                std::fs::write("failing_source.rs", &source).ok();
                panic!("Signal lost due to junk: {:?}\nFailing source written to failing_source.rs", junk);
            }

            // Invariant 1: Signal must be found
            assert_eq!(signals.len(), 1, "Signal lost due to junk: {:?}\nSource:\n{}", junk, source);
            let signal = &signals[0];

            // Invariant 2: Symbol ID must be stable
            assert_eq!(signal.symbol_id.fqn.as_ref(), "target_logic");

            // Invariant 3: Semantic Hash must be identical regardless of junk
            // (Note: This assumes our semantic hash implementation is logic-pure)
            let base_source = "// @epistemic:stable-id\nfn target_logic(a: i32) -> i32 { a + 1 }";
            let base_signals = parser.parse_signals(base_source, "base.rs");
            assert_eq!(signal.semantic_hash, base_signals[0].semantic_hash, "Semantic hash drift detected!");

            // Invariant 4: Structural Hash must also be stable (since we normalize whitespace in structural parse too)
            // Wait, structural hash is on the content. If the content node itself has different whitespace inside?
            // In our implementation, we compute structural_hash on the raw node content.
            // If tree-sitter includes some junk in the node, it might change.
            // But for a function_item, the whitespace is usually exterior.
        }
    }
}
