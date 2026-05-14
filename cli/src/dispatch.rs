//! # Structural Dispatch Module (v1.2.5)
//!
//! Three commands:
//!   run     — parse → normalize → seal
//!   graph   — extract + output unified dependency graph
//!   verify  — seal integrity + drift detection

use crate::term::*;
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use vantage_core::invariant_engine::engine;
use vantage_core::normalizer::Normalizer;
use vantage_core::VANTAGE_VERSION;
use vantage_types::invariant_spec::*;
use vantage_types::intent::IntentInvariant;
use vantage_types::symbol::SymbolKind;

/// Walk a path and collect all recognized source files (deterministic order).
fn collect_source_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
        return files;
    }

    let walker = WalkBuilder::new(path)
        .sort_by_file_path(|a, b| a.cmp(b))
        .git_ignore(true)
        .git_global(true)
        .hidden(true)
        .filter_entry(|entry| {
            let name = entry.path().file_name().and_then(|n| n.to_str()).unwrap_or("");
            !matches!(name, ".git" | "venv" | ".venv" | "node_modules" | "target" | "__pycache__" | ".pytest_cache" | ".mypy_cache" | "dist" | "build")
        })
        .build();

    for entry in walker.flatten() {
        let p = entry.path();
        if p.is_file() {
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if matches!(ext, "rs" | "py" | "rb" | "js" | "jsx" | "ts" | "tsx") {
                    files.push(p.to_path_buf());
                }
            }
        }
    }

    files
}

/// Run: parse → normalize → optionally seal.
pub fn execute_run(path: PathBuf, use_json: bool, dry_run: bool) -> Result<()> {
    let files = collect_source_files(&path);

    if files.is_empty() {
        anyhow::bail!("No supported source files found");
    }

    let mut total_nodes = 0;
    let mut results = Vec::new();

    for file in &files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut normalizer = Normalizer::new();
        let file_path = file.to_string_lossy().to_string();

        match normalizer.run(&source, &file_path) {
            Ok(graph) => {
                let count = graph.nodes.len();
                total_nodes += count;
                results.push(json!({
                    "file": file_path,
                    "language": graph.source_language.as_str(),
                    "nodes": count,
                }));
            }
            Err(e) => {
                if use_json {
                    results.push(json!({
                        "file": file_path,
                        "error": e,
                    }));
                } else {
                    eprintln!("  [!] {}: {}", file_path, e);
                }
            }
        }
    }

    // Create seal unless dry_run
    if !dry_run {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("Time drift")?
            .as_secs();

        let seal_data = json!({
            "v": VANTAGE_VERSION,
            "ts": ts,
            "files": results,
            "total_nodes": total_nodes,
        });

        let seal_path = if path.is_dir() {
            path.join("VANTAGE.SEAL")
        } else {
            path.parent().unwrap_or(Path::new(".")).join("VANTAGE.SEAL")
        };

        std::fs::write(&seal_path, serde_json::to_string_pretty(&seal_data)?)?;

        if !use_json {
            println!("{}", bold!(green!("🛡️  VANTAGE SEAL CREATED")));
            println!("  Path:  {}", seal_path.display());
        }
    }

    if use_json {
        let output = json!({
            "v": VANTAGE_VERSION,
            "status": "ok",
            "files": results,
            "total_nodes": total_nodes,
            "sealed": !dry_run,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", bold!(yellow!("🔍 VANTAGE STRUCTURAL RUN")));
        println!("  Target: {}", path.display());
        println!("  Files:  {}", files.len());
        println!("  Nodes:  {}", total_nodes);
        if dry_run {
            println!("  Mode:   {}", cyan!("dry-run (no seal)"));
        } else {
            println!("  Mode:   {}", green!("sealed"));
        }
    }

    Ok(())
}

/// Graph: extract and output the unified dependency graph.
pub fn execute_graph(path: PathBuf) -> Result<()> {
    let files = collect_source_files(&path);

    if files.is_empty() {
        anyhow::bail!("No supported source files found");
    }

    let mut all_nodes = Vec::new();

    for file in &files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let mut normalizer = Normalizer::new();
        let file_path = file.to_string_lossy().to_string();

        if let Ok(graph) = normalizer.run(&source, &file_path) {
            all_nodes.extend(graph.nodes);
        }
    }

    all_nodes.sort_by(|a, b| a.language.as_str().cmp(b.language.as_str()).then(a.fq_name.cmp(&b.fq_name)));

    let output = json!({
        "v": VANTAGE_VERSION,
        "status": "ok",
        "total_nodes": all_nodes.len(),
        "nodes": all_nodes,
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Built-in default invariants for common architectural guarantees.
fn default_invariants() -> Vec<InvariantSpec> {
    vec![
        InvariantSpec {
            id: "global-has-struct".to_string(),
            description: "Graph must contain at least one Struct node".to_string(),
            rule: InvariantRule::RequiredKind(SymbolKind::Struct),
            scope: InvariantScope::Global,
        },
        InvariantSpec {
            id: "global-has-function".to_string(),
            description: "Graph must contain at least one Function node".to_string(),
            rule: InvariantRule::RequiredKind(SymbolKind::Function),
            scope: InvariantScope::Global,
        },
        InvariantSpec {
            id: "global-hash-stable".to_string(),
            description: "All nodes must have a non-empty structural hash".to_string(),
            rule: InvariantRule::HashStability,
            scope: InvariantScope::Global,
        },
    ]
}

/// Verify: check seal integrity, run invariants, detect drift.
pub fn execute_verify(path: PathBuf, use_json: bool, _deep: bool) -> Result<()> {
    // Locate seal file
    let seal_path = if path.is_dir() {
        path.join("VANTAGE.SEAL")
    } else {
        path.parent().unwrap_or(Path::new(".")).join("VANTAGE.SEAL")
    };

    if !seal_path.exists() {
        anyhow::bail!(
            "No VANTAGE.SEAL found at {}. Run 'kit-vantage run' first.",
            seal_path.display()
        );
    }

    let seal_data = std::fs::read_to_string(&seal_path)
        .context("Failed to read seal file")?;
    let seal: serde_json::Value = serde_json::from_str(&seal_data)?;

    // Re-run structural analysis on the same target
    let files = collect_source_files(&path);
    let mut all_nodes = Vec::new();
    let mut errors = Vec::new();

    for file in &files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("{}: {}", file.display(), e));
                continue;
            }
        };

        let mut normalizer = Normalizer::new();
        let file_path = file.to_string_lossy().to_string();

        match normalizer.run(&source, &file_path) {
            Ok(graph) => all_nodes.extend(graph.nodes),
            Err(e) => errors.push(format!("{}: {}", file_path, e)),
        }
    }

    let current_nodes = all_nodes.len();
    let baseline_nodes = seal["total_nodes"].as_u64().unwrap_or(0) as usize;
    let drift = if current_nodes != baseline_nodes || !errors.is_empty() {
        "drift"
    } else {
        "ok"
    };

    // Create a unified graph for invariant validation
    let combined_graph = vantage_types::uir::UnifiedGraph {
        nodes: all_nodes,
        source_language: vantage_types::uir::Language::Rust,
    };
    let invariants = default_invariants();
    let inv_report = engine::validate_invariants(&combined_graph, &invariants);

    if use_json {
        let output = json!({
            "v": VANTAGE_VERSION,
            "status": drift,
            "baseline_nodes": baseline_nodes,
            "current_nodes": current_nodes,
            "errors": errors,
            "invariants": {
                "total": inv_report.total,
                "passed": inv_report.passed,
                "failed": inv_report.failed,
                "results": inv_report.results.iter().map(|r| json!({
                    "spec_id": r.spec_id,
                    "passed": r.passed,
                    "message": r.message,
                })).collect::<Vec<_>>(),
            },
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{}", bold!(yellow!("🛡️  VANTAGE VERIFY")));
        println!("  Seal:    {}", seal_path.display());
        println!("  Baseline: {} nodes", baseline_nodes);
        println!("  Current:  {} nodes", current_nodes);

        if drift == "drift" {
            println!("  Status:  {}", red!("DRIFT DETECTED"));
        } else {
            println!("  Status:  {}", green!("INTEGRITY OK"));
        }

        // Invariant report
        println!();
        println!("{}", bold!("📋 INVARIANT REPORT"));
        println!("  Total: {} | Passed: {} | Failed: {}",
            inv_report.total, green!(inv_report.passed.to_string()), red!(inv_report.failed.to_string()));
        for r in &inv_report.results {
            let icon = if r.passed { green!("✓") } else { red!("✗") };
            println!("  {} [{}] {} - {}", icon, r.spec_id, r.message, bold!(dim!("")));
        }

        if !errors.is_empty() {
            println!();
            println!("  Errors: {}", errors.len());
            for e in &errors {
                println!("    - {}", e);
            }
        }
    }

    Ok(())
}
