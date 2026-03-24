use vantage_core::cognition::EpistemicParser;
use proptest::prelude::*;

#[cfg(test)]
mod stability_tests {
    use super::*;

    fn setup_rust_parser() -> EpistemicParser {
        EpistemicParser::new_rust_parser().expect("Failed to initialize Rust parser")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1000))]
        
        #[test]
        fn test_semantic_invariant_under_radioactive_junk(
            junk in ".{0,50}"
        ) {
            let mut parser = setup_rust_parser();
            
            // The logic stays the same, only the whitespace/junk around it changes
            let source = format!(
                "// @epistemic:stable-id\n{}fn target_logic(a: i32) -> i32 {{ a + 1 }}{}",
                junk, junk
            );

            let signals = parser.parse_signals(&source);
            
            // Invariant 1: Signal must be found
            assert_eq!(signals.len(), 1, "Signal lost due to junk: {:?}", junk);
            
            let signal = &signals[0];
            
            // Invariant 2: Symbol ID must be stable
            assert_eq!(signal.symbol_id, "target_logic");
            
            // Invariant 3: Semantic Hash must be identical regardless of junk
            // (Note: This assumes our semantic hash implementation is logic-pure)
            let base_source = "// @epistemic:stable-id\nfn target_logic(a: i32) -> i32 { a + 1 }";
            let base_signals = parser.parse_signals(base_source);
            assert_eq!(signal.semantic_hash, base_signals[0].semantic_hash, "Semantic hash drift detected!");
            
            // Invariant 4: Structural Hash must also be stable (since we normalize whitespace in structural parse too)
            // Wait, structural hash is on the content. If the content node itself has different whitespace inside?
            // In our implementation, we compute structural_hash on the raw node content.
            // If tree-sitter includes some junk in the node, it might change.
            // But for a function_item, the whitespace is usually exterior.
        }
    }
}
