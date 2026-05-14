//! # Golden Test Repositories — v1.2.5
//!
//! Controlled reality environments for graph truth validation.
//! Tests are organized by repo and validate:
//!   1. Structural correctness — AST → graph mapping
//!   2. Cross-language consistency — same concept → expected node shape
//!   3. Intent fidelity — vantage: blocks attach to correct nodes
//!   4. Merge stability — multi-file graph topology integrity

use std::fs;
use std::path::{Path, PathBuf};
use vantage_core::normalizer::Normalizer;

/// Root directory for all golden repos (resolved from crate root).
fn repos_root() -> PathBuf {
    let cargo_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    cargo_dir.join("tests").join("repos")
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Walk a directory and return all recognized source files (deterministic order).
fn collect_files(dir: &Path) -> Vec<PathBuf> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    if !dir.exists() {
        return files;
    }

    let walker = WalkBuilder::new(dir)
        .sort_by_file_path(|a, b| a.cmp(b))
        .git_ignore(false)
        .git_global(false)
        .git_exclude(false)
        .hidden(false)
        .build();

    for entry in walker.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "rs" | "py" | "rb" | "js" | "jsx" | "ts" | "tsx") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    files
}

/// Parse a single file and return the number of nodes in the unified graph.
fn parse_file(path: &Path) -> Result<usize, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut normalizer = Normalizer::new();
    let file_path = path.to_string_lossy().to_string();
    let graph = normalizer.run(&source, &file_path)?;
    Ok(graph.nodes.len())
}

/// Parse a file and return all node kinds found.
fn parse_kinds(path: &Path) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut normalizer = Normalizer::new();
    let file_path = path.to_string_lossy().to_string();
    let graph = normalizer.run(&source, &file_path)?;
    let mut kinds: Vec<String> = graph.nodes.iter().map(|n| format!("{:?}", n.kind)).collect();
    kinds.sort();
    Ok(kinds)
}

/// Parse a file and return nodes that have intent annotations.
fn parse_intents(path: &Path) -> Result<Vec<String>, String> {
    let source = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    let mut normalizer = Normalizer::new();
    let file_path = path.to_string_lossy().to_string();
    let graph = normalizer.run(&source, &file_path)?;
    let mut intents: Vec<String> = graph
        .nodes
        .iter()
        .filter_map(|n| n.intent.as_ref().map(|i| format!("{:?}@{}", i.invariant, n.fq_name)))
        .collect();
    intents.sort();
    Ok(intents)
}

/// Parse a repo directory and sum total nodes across all files.
fn parse_repo_total(dir: &Path) -> Result<(usize, usize), String> {
    let files = collect_files(dir);
    let mut total_nodes = 0;
    let mut file_count = 0;
    for f in &files {
        if let Ok(count) = parse_file(f) {
            total_nodes += count;
            file_count += 1;
        }
    }
    Ok((file_count, total_nodes))
}

// ─── Rust Core Repo ─────────────────────────────────────────────────────────

#[test]
fn golden_rust_core_parses() {
    let dir = repos_root().join("rust_core").join("src");
    let files = collect_files(&dir);
    assert!(!files.is_empty(), "rust_core: no source files found");

    let (file_count, node_count) = parse_repo_total(&dir).unwrap();
    assert!(file_count >= 4, "rust_core: expected >=4 files, got {}", file_count);
    assert!(node_count >= 8, "rust_core: expected >=8 nodes, got {}", node_count);
}

#[test]
fn golden_rust_core_intents() {
    let dir = repos_root().join("rust_core").join("src");
    let files = collect_files(&dir);
    let mut found_append_only = false;
    let mut found_thread_safe = false;
    let mut found_deterministic = false;

    for f in &files {
        if let Ok(intents) = parse_intents(f) {
            for i in &intents {
                if i.contains("AppendOnly") { found_append_only = true; }
                if i.contains("ThreadSafe") { found_thread_safe = true; }
                if i.contains("DeterministicOrdering") { found_deterministic = true; }
            }
        }
    }

    assert!(found_append_only, "rust_core: missing AppendOnly intent on Arena");
    assert!(found_thread_safe, "rust_core: missing ThreadSafe intent on MemoryPool");
    assert!(found_deterministic, "rust_core: missing DeterministicOrdering intent on Scheduler");
}

#[test]
fn golden_rust_core_node_kinds() {
    let dir = repos_root().join("rust_core").join("src");
    let files = collect_files(&dir);
    let mut all_kinds = Vec::new();
    for f in &files {
        if let Ok(kinds) = parse_kinds(f) {
            all_kinds.extend(kinds);
        }
    }
    assert!(all_kinds.contains(&"Struct".to_string()), "rust_core: expected Struct nodes");
}

// ─── Python Core Repo ───────────────────────────────────────────────────────

#[test]
fn golden_python_core_parses() {
    let dir = repos_root().join("python_core").join("core");
    let files = collect_files(&dir);
    assert!(!files.is_empty(), "python_core: no source files found");

    let (file_count, node_count) = parse_repo_total(&dir).unwrap();
    assert!(file_count >= 3, "python_core: expected >=3 files, got {}", file_count);
    assert!(node_count >= 6, "python_core: expected >=6 nodes, got {}", node_count);
}

#[test]
fn golden_python_core_intents() {
    let dir = repos_root().join("python_core").join("core");
    let files = collect_files(&dir);
    let mut found_append_only = false;
    let mut found_thread_safe = false;
    let mut found_deterministic = false;

    for f in &files {
        if let Ok(intents) = parse_intents(f) {
            for i in &intents {
                if i.contains("AppendOnly") { found_append_only = true; }
                if i.contains("ThreadSafe") { found_thread_safe = true; }
                if i.contains("DeterministicOrdering") { found_deterministic = true; }
            }
        }
    }

    assert!(found_append_only, "python_core: missing AppendOnly intent on Arena");
    assert!(found_thread_safe, "python_core: missing ThreadSafe intent on MemoryPool");
    assert!(found_deterministic, "python_core: missing DeterministicOrdering intent on Scheduler");
}

#[test]
fn golden_python_core_node_kinds() {
    let dir = repos_root().join("python_core").join("core");
    let files = collect_files(&dir);
    let mut all_kinds = Vec::new();
    for f in &files {
        if let Ok(kinds) = parse_kinds(f) {
            all_kinds.extend(kinds);
        }
    }
    assert!(all_kinds.contains(&"Class".to_string()), "python_core: expected Class nodes");
    assert!(all_kinds.contains(&"Function".to_string()), "python_core: expected Function nodes");
}

// ─── Ruby Core Repo ─────────────────────────────────────────────────────────

#[test]
fn golden_ruby_core_parses() {
    let dir = repos_root().join("ruby_core").join("lib");
    let files = collect_files(&dir);
    assert!(!files.is_empty(), "ruby_core: no source files found");

    let (file_count, node_count) = parse_repo_total(&dir).unwrap();
    assert!(file_count >= 3, "ruby_core: expected >=3 files, got {}", file_count);
    assert!(node_count >= 6, "ruby_core: expected >=6 nodes, got {}", node_count);
}

#[test]
fn golden_ruby_core_intents() {
    let dir = repos_root().join("ruby_core").join("lib");
    let files = collect_files(&dir);
    let mut found_append_only = false;
    let mut found_thread_safe = false;
    let mut found_deterministic = false;

    for f in &files {
        if let Ok(intents) = parse_intents(f) {
            for i in &intents {
                if i.contains("AppendOnly") { found_append_only = true; }
                if i.contains("ThreadSafe") { found_thread_safe = true; }
                if i.contains("DeterministicOrdering") { found_deterministic = true; }
            }
        }
    }

    assert!(found_append_only, "ruby_core: missing AppendOnly intent on Arena");
    assert!(found_thread_safe, "ruby_core: missing ThreadSafe intent on MemoryPool");
    assert!(found_deterministic, "ruby_core: missing DeterministicOrdering intent on Scheduler");
}

#[test]
fn golden_ruby_core_node_kinds() {
    let dir = repos_root().join("ruby_core").join("lib");
    let files = collect_files(&dir);
    let mut all_kinds = Vec::new();
    for f in &files {
        if let Ok(kinds) = parse_kinds(f) {
            all_kinds.extend(kinds);
        }
    }
    assert!(all_kinds.contains(&"Class".to_string()), "ruby_core: expected Class nodes");
}

// ─── TSX Frontend Repo ──────────────────────────────────────────────────────

#[test]
fn golden_tsx_frontend_parses() {
    let dir = repos_root().join("tsx_frontend").join("src");
    let files = collect_files(&dir);
    assert!(!files.is_empty(), "tsx_frontend: no source files found");

    let (file_count, node_count) = parse_repo_total(&dir).unwrap();
    assert!(file_count >= 3, "tsx_frontend: expected >=3 files, got {}", file_count);
    assert!(node_count >= 6, "tsx_frontend: expected >=6 nodes, got {}", node_count);
}

#[test]
fn golden_tsx_frontend_node_kinds() {
    let dir = repos_root().join("tsx_frontend").join("src");
    let files = collect_files(&dir);
    let mut all_kinds = Vec::new();
    for f in &files {
        if let Ok(kinds) = parse_kinds(f) {
            all_kinds.extend(kinds);
        }
    }
    assert!(all_kinds.contains(&"Function".to_string()), "tsx_frontend: expected Function nodes");
}

// ─── Mixed System Repo ──────────────────────────────────────────────────────

#[test]
fn golden_mixed_system_all_languages() {
    let dir = repos_root().join("mixed_system");
    let files = collect_files(&dir);
    assert!(!files.is_empty(), "mixed_system: no source files found");

    let extensions: Vec<String> = files
        .iter()
        .filter_map(|p| p.extension().and_then(|s| s.to_str()).map(String::from))
        .collect();

    assert!(extensions.contains(&"rs".to_string()), "mixed_system: missing Rust files");
    assert!(extensions.contains(&"py".to_string()), "mixed_system: missing Python files");
    assert!(extensions.contains(&"tsx".to_string()), "mixed_system: missing TSX files");
    assert!(extensions.contains(&"rb".to_string()), "mixed_system: missing Ruby files");
}

#[test]
fn golden_mixed_system_parses() {
    let dir = repos_root().join("mixed_system");
    let (file_count, node_count) = parse_repo_total(&dir).unwrap();
    assert!(file_count >= 5, "mixed_system: expected >=5 files, got {}", file_count);
    assert!(node_count >= 8, "mixed_system: expected >=8 nodes, got {}", node_count);
}

#[test]
fn golden_mixed_system_intents() {
    let dir = repos_root().join("mixed_system");
    let files = collect_files(&dir);
    let mut found_intents = Vec::new();

    for f in &files {
        if let Ok(intents) = parse_intents(f) {
            found_intents.extend(intents);
        }
    }

    assert!(
        found_intents.iter().any(|i| i.contains("AppendOnly")),
        "mixed_system: missing AppendOnly intent on Rust Arena"
    );
    assert!(
        found_intents.iter().any(|i| i.contains("Stateless")),
        "mixed_system: missing Stateless intent on Python server"
    );
    assert!(
        found_intents.iter().any(|i| i.contains("NoSideEffects") || i.contains("Idempotent")),
        "mixed_system: missing NoSideEffects/Idempotent intent"
    );
}

// ─── Cross-language Concept Parity ──────────────────────────────────────────

#[test]
fn golden_cross_language_concept_parity() {
    // The concept "Arena" exists as:
    //   - Struct in Rust
    //   - Class in Python
    //   - Class in Ruby
    // They should NOT be merged — each is its own node.

    let rust_dir = repos_root().join("rust_core").join("src");
    let py_dir = repos_root().join("python_core").join("core");
    let rb_dir = repos_root().join("ruby_core").join("lib");

    let rust_arena = rust_dir.join("arena.rs");
    let py_arena = py_dir.join("arena.py");
    let rb_arena = rb_dir.join("arena.rb");

    for path in &[&rust_arena, &py_arena, &rb_arena] {
        assert!(
            path.exists(),
            "Arena file missing: {}",
            path.display()
        );
    }

    // Each parses independently without crash
    for path in &[&rust_arena, &py_arena, &rb_arena] {
        let result = parse_file(path);
        assert!(
            result.is_ok(),
            "Failed to parse Arena: {}",
            path.display()
        );
    }
}

// ─── Invariant Validation ───────────────────────────────────────────────────

#[test]
fn golden_rust_invariants_pass() {
    let dir = repos_root().join("rust_core").join("src");
    let files = collect_files(&dir);
    let mut all_nodes = Vec::new();
    for f in &files {
        if let Ok(source) = fs::read_to_string(f) {
            let mut n = Normalizer::new();
            if let Ok(graph) = n.run(&source, &f.to_string_lossy()) {
                all_nodes.extend(graph.nodes);
            }
        }
    }

    let graph = vantage_types::uir::UnifiedGraph {
        nodes: all_nodes,
        source_language: vantage_types::uir::Language::Rust,
    };

    let specs = vec![
        vantage_types::invariant_spec::InvariantSpec {
            id: "rust-has-struct".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::RequiredKind(
                vantage_types::symbol::SymbolKind::Struct,
            ),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
        vantage_types::invariant_spec::InvariantSpec {
            id: "rust-has-intent".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::RequiredIntent(
                vantage_types::intent::IntentInvariant::AppendOnly,
            ),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
        vantage_types::invariant_spec::InvariantSpec {
            id: "rust-node-count".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::NodeCountMin(8),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
    ];

    let report = vantage_core::invariant_engine::engine::validate_invariants(&graph, &specs);
    assert_eq!(report.failed, 0, "Rust invariants failed: {:?}",
        report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
}

#[test]
fn golden_python_invariants_pass() {
    let dir = repos_root().join("python_core").join("core");
    let files = collect_files(&dir);
    let mut all_nodes = Vec::new();
    for f in &files {
        if let Ok(source) = fs::read_to_string(f) {
            let mut n = Normalizer::new();
            if let Ok(graph) = n.run(&source, &f.to_string_lossy()) {
                all_nodes.extend(graph.nodes);
            }
        }
    }

    let graph = vantage_types::uir::UnifiedGraph {
        nodes: all_nodes,
        source_language: vantage_types::uir::Language::Python,
    };

    let specs = vec![
        vantage_types::invariant_spec::InvariantSpec {
            id: "py-has-class".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::RequiredKind(
                vantage_types::symbol::SymbolKind::Class,
            ),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
        vantage_types::invariant_spec::InvariantSpec {
            id: "py-has-intent".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::RequiredIntent(
                vantage_types::intent::IntentInvariant::ThreadSafe,
            ),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
    ];

    let report = vantage_core::invariant_engine::engine::validate_invariants(&graph, &specs);
    assert_eq!(report.failed, 0, "Python invariants failed: {:?}",
        report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
}

#[test]
fn golden_mixed_invariants_pass() {
    let dir = repos_root().join("mixed_system");
    let files = collect_files(&dir);
    let mut all_nodes = Vec::new();
    for f in &files {
        if let Ok(source) = fs::read_to_string(f) {
            let mut n = Normalizer::new();
            if let Ok(graph) = n.run(&source, &f.to_string_lossy()) {
                all_nodes.extend(graph.nodes);
            }
        }
    }

    let graph = vantage_types::uir::UnifiedGraph {
        nodes: all_nodes,
        source_language: vantage_types::uir::Language::Rust,
    };

    let specs = vec![
        vantage_types::invariant_spec::InvariantSpec {
            id: "mixed-node-count".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::NodeCountMin(8),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
        vantage_types::invariant_spec::InvariantSpec {
            id: "mixed-has-intent".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::RequiredIntent(
                vantage_types::intent::IntentInvariant::Stateless,
            ),
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
        vantage_types::invariant_spec::InvariantSpec {
            id: "mixed-cross-lang-parity".to_string(),
            description: "".to_string(),
            rule: vantage_types::invariant_spec::InvariantRule::HashStability,
            scope: vantage_types::invariant_spec::InvariantScope::Global,
        },
    ];

    let report = vantage_core::invariant_engine::engine::validate_invariants(&graph, &specs);
    assert_eq!(report.failed, 0, "Mixed system invariants failed: {:?}",
        report.results.iter().filter(|r| !r.passed).collect::<Vec<_>>());
}

// ─── Drift Detection ────────────────────────────────────────────────────────

#[test]
fn golden_drift_identical_reports_no_drift() {
    use vantage_types::uir::{Language, UnifiedNode, Visibility};
    use vantage_types::SymbolId;

    let node = UnifiedNode {
        id: SymbolId::new("test::Node"),
        fq_name: "test::Node".to_string(),
        language: Language::Rust,
        kind: vantage_types::symbol::SymbolKind::Struct,
        visibility: Visibility::Public,
        dependencies: vec![],
        file: "test.rs".to_string(),
        line: 1,
        structural_hash: "abc".to_string(),
        normalized_hash: "abc".to_string(),
        intent: None,
    };

    let graph = vantage_types::uir::UnifiedGraph {
        nodes: vec![node],
        source_language: Language::Rust,
    };

    let report = vantage_core::drift_engine::engine::quick_check(&graph, &graph);
    assert!(!report.has_drift, "identical graph should have no drift");
    assert!(report.added_nodes.is_empty());
    assert!(report.removed_nodes.is_empty());
}

#[test]
fn golden_drift_node_removed_detected() {
    use vantage_types::uir::{Language, UnifiedNode, Visibility};
    use vantage_types::SymbolId;

    let dir = repos_root().join("rust_core").join("src");
    let nodes = parse_all_nodes(&dir);

    let current = vantage_types::uir::UnifiedGraph {
        nodes: nodes.clone(),
        source_language: Language::Rust,
    };

    // Simulate baseline with extra node
    let extra = UnifiedNode {
        id: SymbolId::new("ghost::Node"),
        fq_name: "ghost::Node".to_string(),
        language: Language::Rust,
        kind: vantage_types::symbol::SymbolKind::Struct,
        visibility: Visibility::Public,
        dependencies: vec![],
        file: "ghost.rs".to_string(),
        line: 1,
        structural_hash: "xyz".to_string(),
        normalized_hash: "xyz".to_string(),
        intent: None,
    };
    let mut baseline_nodes = nodes.clone();
    baseline_nodes.push(extra);

    let baseline = vantage_types::uir::UnifiedGraph {
        nodes: baseline_nodes,
        source_language: Language::Rust,
    };

    let report = vantage_core::drift_engine::engine::quick_check(&baseline, &current);
    assert!(report.has_drift, "baseline had extra node, drift should be detected");
    assert_eq!(report.removed_nodes.len(), 1, "should detect 1 removed node");
    assert_eq!(report.removed_nodes[0].fq_name, "ghost::Node");
}

#[test]
fn golden_drift_node_added_detected() {
    let dir = repos_root().join("rust_core").join("src");
    let nodes = parse_all_nodes(&dir);

    let baseline = vantage_types::uir::UnifiedGraph {
        nodes: nodes.clone(),
        source_language: vantage_types::uir::Language::Rust,
    };

    // Simulate current with extra node
    let mut current_nodes = nodes.clone();
    current_nodes.push(vantage_types::uir::UnifiedNode {
        id: vantage_types::SymbolId::new("new::Symbol"),
        fq_name: "new::Symbol".to_string(),
        language: vantage_types::uir::Language::Rust,
        kind: vantage_types::symbol::SymbolKind::Function,
        visibility: vantage_types::uir::Visibility::Public,
        dependencies: vec![],
        file: "new.rs".to_string(),
        line: 1,
        structural_hash: "xyz".to_string(),
        normalized_hash: "xyz".to_string(),
        intent: None,
    });
    let current = vantage_types::uir::UnifiedGraph {
        nodes: current_nodes,
        source_language: vantage_types::uir::Language::Rust,
    };

    let report = vantage_core::drift_engine::engine::quick_check(&baseline, &current);
    assert!(report.has_drift);
    assert_eq!(report.added_nodes.len(), 1);
    assert_eq!(report.added_nodes[0].fq_name, "new::Symbol");
}

/// Helper to parse all nodes from a repo directory.
fn parse_all_nodes(dir: &Path) -> Vec<vantage_types::uir::UnifiedNode> {
    let files = collect_files(dir);
    let mut all_nodes = Vec::new();
    for f in &files {
        if let Ok(source) = fs::read_to_string(f) {
            let mut n = Normalizer::new();
            if let Ok(graph) = n.run(&source, &f.to_string_lossy()) {
                all_nodes.extend(graph.nodes);
            }
        }
    }
    all_nodes
}

// ─── Edge Cases ─────────────────────────────────────────────────────────────

#[test]
fn golden_empty_file_safe() {
    let source = "";
    let mut normalizer = Normalizer::new();
    let result = normalizer.run(source, "empty.rs");
    assert!(result.is_err() || result.unwrap().nodes.is_empty(),
        "empty file should produce 0 nodes or error");
}

#[test]
fn golden_invalid_syntax_safe() {
    let source = "this is not valid code {{{{{";
    let mut normalizer = Normalizer::new();
    let result = normalizer.run(source, "broken.rs");
    // Should not panic — parser must handle gracefully
    assert!(result.is_ok() || result.is_err(),
        "invalid syntax must not panic");
}

#[test]
fn golden_no_intent_fallback() {
    let source = "fn foo() {}";
    let mut normalizer = Normalizer::new();
    let graph = normalizer.run(source, "no_intent.rs").unwrap();
    for node in &graph.nodes {
        assert!(node.intent.is_none(),
            "nodes without vantage: comments must have None intent");
    }
}
