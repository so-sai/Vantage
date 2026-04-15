//! Vantage v1.2.4 Edge Topology Audit Report
//!
//! Analysis of O(n²) behavior emerging at >500 nodes.
//! Generated from benchmark_suite.rs results.

use std::time::Instant;
use vantage_core::parser::EpistemicParser;
use vantage_types::SymbolGraphDTO;

fn make_functions(count: usize) -> String {
    let mut source = String::new();
    for i in 0..count {
        source.push_str(&format!(
            "// @epistemic:audit-{}\nfn func_{}() {{ let x = {}; }}\n",
            i, i, i
        ));
    }
    source
}

mod layer_breakdown {
    use super::*;

    #[test]
    fn parse_overhead_100() {
        let source = make_functions(100);

        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        // Time: Full parse_with_graph
        let start = Instant::now();
        let _dto = parser.parse_with_graph(&source, "a.rs").1.to_dto();
        let parse_ms = start.elapsed().as_millis();

        // Reset and time: Just signals (no graph)
        let mut parser2 = EpistemicParser::new_rust_parser().unwrap();
        let start2 = Instant::now();
        let _signals = parser2.parse_signals(&source, "a.rs");
        let signal_ms = start2.elapsed().as_millis();

        println!(
            "[100] parse_with_graph: {}ms, signals_only: {}ms",
            parse_ms, signal_ms
        );
    }

    #[test]
    fn parse_overhead_500() {
        let source = make_functions(500);

        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        let start = Instant::now();
        let _dto = parser.parse_with_graph(&source, "a.rs").1.to_dto();
        let parse_ms = start.elapsed().as_millis();

        let mut parser2 = EpistemicParser::new_rust_parser().unwrap();
        let start2 = Instant::now();
        let _signals = parser2.parse_signals(&source, "a.rs");
        let signal_ms = start2.elapsed().as_millis();

        println!(
            "[500] parse_with_graph: {}ms, signals_only: {}ms",
            parse_ms, signal_ms
        );
    }

    #[test]
    fn parse_overhead_1000() {
        let source = make_functions(1000);

        let mut parser = EpistemicParser::new_rust_parser().unwrap();

        let start = Instant::now();
        let _dto = parser.parse_with_graph(&source, "a.rs").1.to_dto();
        let parse_ms = start.elapsed().as_millis();

        let mut parser2 = EpistemicParser::new_rust_parser().unwrap();
        let start2 = Instant::now();
        let _signals = parser2.parse_signals(&source, "a.rs");
        let signal_ms = start2.elapsed().as_millis();

        println!(
            "[1000] parse_with_graph: {}ms, signals_only: {}ms",
            parse_ms, signal_ms
        );
    }
}

mod anchor_cost {
    use super::*;

    #[test]
    fn anchor_resolution_cost() {
        // With 1 anchor per function, N functions = N anchor resolutions
        // Each resolution does full tree traversal (find_first_solid_after_byte)

        let sources = vec![
            ("100 anchors", make_functions(100)),
            ("500 anchors", make_functions(500)),
            ("1000 anchors", make_functions(1000)),
        ];

        for (label, source) in sources {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();
            let start = Instant::now();
            let signals = parser.parse_signals(&source, "a.rs");
            let elapsed = start.elapsed().as_millis();

            println!("[{}] {}ms, {} signals found", label, elapsed, signals.len());
        }
    }
}

mod graph_edge_cost {
    use super::*;

    fn make_call_graph(depth: usize, width: usize) -> String {
        let mut source = String::new();

        // Root calls all width functions
        for i in 0..width {
            source.push_str(&format!(
                "// @epistemic:root-{}\nfn root() {{ call_{}(); call_{}(); }}\n",
                i,
                i % width,
                (i + 1) % width
            ));
        }

        // Each leaf
        for i in 0..width {
            source.push_str(&format!(
                "// @epistemic:leaf-{}\nfn call_{}() {{ }}\n",
                i, i
            ));
        }

        source
    }

    #[test]
    fn edge_construction_cost() {
        // Test: pure edge construction (no anchor resolution)
        // Each function has @epistemic, so there's still anchor cost
        // but fewer anchors = less tree traversal

        let test_cases = vec![(10, 10), (50, 50), (100, 100)];

        for (depth, width) in test_cases {
            let source = make_call_graph(depth, width);
            let mut parser = EpistemicParser::new_rust_parser().unwrap();

            let start = Instant::now();
            let (signals, graph) = parser.parse_with_graph(&source, "a.rs");
            let elapsed = start.elapsed().as_millis();

            let edge_count: usize = graph.nodes.values().map(|n| n.dependencies.len()).sum();

            println!(
                "[{}x{}] {}ms, {} signals, {} edges",
                depth,
                width,
                elapsed,
                signals.len(),
                edge_count
            );
        }
    }
}

mod tree_traversal_cost {
    use super::*;

    #[test]
    fn solid_target_resolution() {
        // Each @epistemic: tag requires find_first_solid_after_byte
        // This walks the ENTIRE tree from root for each anchor
        // N anchors × O(tree_size) = O(n²)

        let sources = vec![
            ("50 solid", make_functions(50)),
            ("100 solid", make_functions(100)),
            ("200 solid", make_functions(200)),
        ];

        for (label, source) in sources {
            let mut parser = EpistemicParser::new_rust_parser().unwrap();

            // parse_signals does extraction + solid target resolution
            let start = Instant::now();
            let signals = parser.parse_signals(&source, "a.rs");
            let elapsed = start.elapsed().as_millis();

            println!("[{}] {}ms for {} signals", label, elapsed, signals.len());
        }
    }
}
