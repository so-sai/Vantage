//! Vantage v1.2.4 Performance Benchmark Suite
//!
//! Measures system phase space: performance boundaries, scaling characteristics, and cost model.
//! Run with: cargo test --test benchmark_suite -- --nocapture

use std::time::Instant;
use vantage_core::parser::EpistemicParser;
use vantage_types::{IdentityAnchor, NodeId, SemanticRole, SymbolGraphDTO, SymbolId};

fn make_functions(count: usize) -> String {
    let mut source = String::new();
    for i in 0..count {
        source.push_str(&format!(
            "// @epistemic:bench-{}\nfn func_{}() {{ let x = {}; }}\n",
            i, i, i
        ));
    }
    source
}

fn make_dependencies(depth: usize, breadth: usize) -> String {
    let mut source = String::new();

    // Root level - each root calls a leaf
    for i in 0..breadth {
        source.push_str(&format!(
            "// @epistemic:bench-root-{}\nfn root_{}() {{ leaf_{}(); }}\n",
            i, i, i
        ));
    }

    // Leaf level - independent functions
    for i in 0..breadth {
        for j in 0..depth {
            source.push_str(&format!(
                "// @epistemic:bench-leaf-{i}-{j}\nfn leaf_{i}_{j}() {{ }}\n"
            ));
        }
    }

    source
}

fn parse_and_time(source: &str) -> (SymbolGraphDTO, u128) {
    let mut parser = EpistemicParser::new_rust_parser().unwrap();
    let start = Instant::now();
    let dto = parser.parse_with_graph(source, "bench.rs").1.to_dto();
    let elapsed = start.elapsed().as_millis();
    (dto, elapsed)
}

mod linear_scaling {
    use super::*;

    #[test]
    fn parse_100_linear() {
        let source = make_functions(100);
        let (dto, elapsed) = parse_and_time(&source);
        println!("[100 nodes] {}ms, {} graph nodes", elapsed, dto.nodes.len());
        assert!(elapsed < 500, "100 nodes: {}ms", elapsed);
    }

    #[test]
    fn parse_500_linear() {
        let source = make_functions(500);
        let (dto, elapsed) = parse_and_time(&source);
        println!("[500 nodes] {}ms, {} graph nodes", elapsed, dto.nodes.len());
        assert!(elapsed < 5000, "500 nodes: {}ms", elapsed);
    }

    #[test]
    fn parse_1000_linear() {
        let source = make_functions(1000);
        let (dto, elapsed) = parse_and_time(&source);
        println!(
            "[1000 nodes] {}ms, {} graph nodes",
            elapsed,
            dto.nodes.len()
        );
        assert!(elapsed < 10000, "1000 nodes: {}ms", elapsed);
    }

    #[test]
    fn parse_2500_linear() {
        let source = make_functions(2500);
        let (dto, elapsed) = parse_and_time(&source);
        println!(
            "[2500 nodes] {}ms, {} graph nodes",
            elapsed,
            dto.nodes.len()
        );
        assert!(elapsed < 60000, "2500 nodes: {}ms", elapsed);
    }
}

mod dependency_scaling {
    use super::*;

    #[test]
    fn deep_graph_10x10() {
        let source = make_dependencies(10, 10);
        let (dto, elapsed) = parse_and_time(&source);
        println!(
            "[10x10 graph] {}ms, {} nodes, {} edges",
            elapsed,
            dto.nodes.len(),
            dto.nodes
                .iter()
                .map(|n| n.dependencies.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn deep_graph_20x20() {
        let source = make_dependencies(20, 20);
        let (dto, elapsed) = parse_and_time(&source);
        println!(
            "[20x20 graph] {}ms, {} nodes, {} edges",
            elapsed,
            dto.nodes.len(),
            dto.nodes
                .iter()
                .map(|n| n.dependencies.len())
                .sum::<usize>()
        );
    }

    #[test]
    fn deep_graph_50x50() {
        let source = make_dependencies(50, 50);
        let (dto, elapsed) = parse_and_time(&source);
        println!(
            "[50x50 graph] {}ms, {} nodes, {} edges",
            elapsed,
            dto.nodes.len(),
            dto.nodes
                .iter()
                .map(|n| n.dependencies.len())
                .sum::<usize>()
        );
    }
}

mod edge_cost {
    use super::*;

    fn nodeid_generate_bench(count: usize) -> u128 {
        let start = Instant::now();
        let parent = Some(NodeId(1));
        let role = SemanticRole::LetBinding;

        for i in 0..count {
            let anchor = IdentityAnchor::Binding(SymbolId::new(&format!("symbol_{}", i)));
            let _ = NodeId::generate(parent, role, &anchor);
        }

        start.elapsed().as_millis()
    }

    #[test]
    fn nodeid_1k_generates() {
        let elapsed = nodeid_generate_bench(1000);
        println!("[1k NodeId generates] {}ms", elapsed);
        assert!(elapsed < 1000, "1k generates: {}ms", elapsed);
    }

    #[test]
    fn nodeid_10k_generates() {
        let elapsed = nodeid_generate_bench(10000);
        println!("[10k NodeId generates] {}ms", elapsed);
        assert!(elapsed < 5000, "10k generates: {}ms", elapsed);
    }
}

mod serialization_cost {
    use super::*;

    #[test]
    fn json_serialize_100() {
        let source = make_functions(100);
        let (dto, _) = parse_and_time(&source);

        let start = Instant::now();
        let json = serde_json::to_string(&dto).unwrap();
        let elapsed = start.elapsed().as_millis();

        println!("[100 nodes serialize] {}ms, {} bytes", elapsed, json.len());
        assert!(elapsed < 100, "100 serialize: {}ms", elapsed);
    }

    #[test]
    fn json_serialize_1k() {
        let source = make_functions(1000);
        let (dto, _) = parse_and_time(&source);

        let start = Instant::now();
        let json = serde_json::to_string(&dto).unwrap();
        let elapsed = start.elapsed().as_millis();

        println!("[1k nodes serialize] {}ms, {} bytes", elapsed, json.len());
        assert!(elapsed < 2000, "1k serialize: {}ms", elapsed);
    }

    #[test]
    fn json_roundtrip_100() {
        let source = make_functions(100);
        let (dto, parse_elapsed) = parse_and_time(&source);

        let ser_start = Instant::now();
        let json = serde_json::to_string(&dto).unwrap();
        let ser_elapsed = ser_start.elapsed().as_millis();

        let de_start = Instant::now();
        let _reconstructed: SymbolGraphDTO = serde_json::from_str(&json).unwrap();
        let de_elapsed = de_start.elapsed().as_millis();

        println!(
            "[100 nodes roundtrip] parse:{}ms serialize:{}ms deserialize:{}ms",
            parse_elapsed, ser_elapsed, de_elapsed
        );
    }

    #[test]
    fn json_roundtrip_1k() {
        let source = make_functions(1000);
        let (dto, parse_elapsed) = parse_and_time(&source);

        let ser_start = Instant::now();
        let json = serde_json::to_string(&dto).unwrap();
        let ser_elapsed = ser_start.elapsed().as_millis();

        let de_start = Instant::now();
        let _reconstructed: SymbolGraphDTO = serde_json::from_str(&json).unwrap();
        let de_elapsed = de_start.elapsed().as_millis();

        println!(
            "[1k nodes roundtrip] parse:{}ms serialize:{}ms deserialize:{}ms",
            parse_elapsed, ser_elapsed, de_elapsed
        );
    }
}

mod memory_baseline {
    use super::*;

    #[test]
    fn empty_parse() {
        let start = Instant::now();
        let mut parser = EpistemicParser::new_rust_parser().unwrap();
        let dto = parser.parse_with_graph("", "empty.rs").1.to_dto();
        let elapsed = start.elapsed().as_millis();

        println!(
            "[empty parse] {}ms, {} bytes",
            elapsed,
            std::mem::size_of_val(&dto)
        );
    }
}
