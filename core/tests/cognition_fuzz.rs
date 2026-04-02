use proptest::prelude::*;
use vantage_core::EpistemicParser;

fn junk_strategy() -> impl Strategy<Value = String> {
    // Only test with realistic whitespace: spaces, tabs, newlines
    prop::string::string_regex(r"([ \t\n\r]|//[ -~]*\n){0,5}").unwrap()
}

proptest! {
    #[test]
    fn fuzz_void_resilience(junk in junk_strategy()) {
        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        let source = format!(
            "// @epistemic:FUZZ-ID\n{}\nfn target() {{}}",
            junk
        );

        let signals = parser.parse_signals(&source, "fuzz_test.rs");

        if signals.is_empty() {
            panic!("Parser blinded by junk: {:?}", junk);
        }

        assert_eq!(signals[0].uuid, "FUZZ-ID");
    }
}
