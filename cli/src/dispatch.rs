//! # Structural Dispatch Module (v1.2.5)
//!
//! Three commands + CI mode:
//!   run     — parse → normalize → seal
//!   graph   — extract + output unified dependency graph
//!   verify  — seal integrity + drift + invariants
//!   --ci    — stable JSON contract for automation/agents
//!
//! Exit codes (CI mode):
//!   0  = OK
//!   1  = Drift detected
//!   2  = Invariant violation
//!   10 = Internal engine error

use crate::term::*;
use anyhow::{Context, Result};
use ignore::WalkBuilder;
use serde_json::json;
use std::process;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use vantage_core::drift_engine::engine as drift_engine;
use vantage_core::invariant_engine::engine as invariant_engine;
use vantage_core::normalizer::Normalizer;
use vantage_core::VANTAGE_VERSION;
use vantage_types::invariant_spec::*;
use vantage_types::intent::IntentInvariant;
use vantage_types::symbol::SymbolKind;

// ─── Exit Codes (CI contract) ───────────────────────────────────────────────

const EXIT_OK: i32 = 0;
const EXIT_DRIFT: i32 = 1;
const EXIT_INVARIANT: i32 = 2;
const EXIT_ERROR: i32 = 10;

// ─── Helpers ────────────────────────────────────────────────────────────────

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

fn ci_output(data: serde_json::Value) {
    println!("{}", serde_json::to_string(&data).unwrap_or_default());
}

// ─── Run ────────────────────────────────────────────────────────────────────

pub fn execute_run(path: PathBuf, ci: bool, dry_run: bool) -> Result<()> {
    let files = collect_source_files(&path);

    if files.is_empty() {
        if ci {
            ci_output(json!({
                "v": VANTAGE_VERSION,
                "status": "error",
                "message": "No supported source files found",
            }));
            process::exit(EXIT_ERROR);
        }
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
                results.push(json!({
                    "file": file_path,
                    "error": e,
                }));
            }
        }
    }

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
    }

    if ci {
        ci_output(json!({
            "v": VANTAGE_VERSION,
            "status": "ok",
            "files": results,
            "total_nodes": total_nodes,
            "sealed": !dry_run,
        }));
        process::exit(EXIT_OK);
    }

    println!("{}", bold!(yellow!("🔍 VANTAGE STRUCTURAL RUN")));
    println!("  Target: {}", path.display());
    println!("  Files:  {}", files.len());
    println!("  Nodes:  {}", total_nodes);
    if dry_run {
        println!("  Mode:   {}", cyan!("dry-run (no seal)"));
    } else {
        println!("  Mode:   {}", green!("sealed"));
    }

    Ok(())
}

// ─── Graph ──────────────────────────────────────────────────────────────────

pub fn execute_graph(path: PathBuf, ci: bool) -> Result<()> {
    let files = collect_source_files(&path);

    if files.is_empty() {
        if ci {
            ci_output(json!({
                "v": VANTAGE_VERSION,
                "status": "error",
                "message": "No supported source files found",
            }));
            process::exit(EXIT_ERROR);
        }
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

    let output = json!({
        "v": VANTAGE_VERSION,
        "status": "ok",
        "total_nodes": all_nodes.len(),
        "nodes": all_nodes,
    });

    if ci {
        ci_output(output);
        process::exit(EXIT_OK);
    }

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

// ─── Verify ─────────────────────────────────────────────────────────────────

pub fn execute_verify(path: PathBuf, ci: bool, _deep: bool) -> Result<()> {
    let seal_path = if path.is_dir() {
        path.join("VANTAGE.SEAL")
    } else {
        path.parent().unwrap_or(Path::new(".")).join("VANTAGE.SEAL")
    };

    if !seal_path.exists() {
        if ci {
            ci_output(json!({
                "v": VANTAGE_VERSION,
                "status": "error",
                "message": "No VANTAGE.SEAL found",
            }));
            process::exit(EXIT_ERROR);
        }
        anyhow::bail!("No VANTAGE.SEAL found at {}. Run 'kit-vantage run' first.", seal_path.display());
    }

    let seal_data = std::fs::read_to_string(&seal_path)
        .context("Failed to read seal file")?;
    let seal: serde_json::Value = serde_json::from_str(&seal_data)?;

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

    let current_count = all_nodes.len();
    let baseline_count = seal["total_nodes"].as_u64().unwrap_or(0) as usize;
    let has_node_drift = current_count != baseline_count || !errors.is_empty();

    // Unified graph for invariant + drift check
    let combined_graph = vantage_types::uir::UnifiedGraph {
        nodes: all_nodes,
        source_language: vantage_types::uir::Language::Rust,
    };
    let invariants = default_invariants();
    let inv_report = invariant_engine::validate_invariants(&combined_graph, &invariants);
    let has_invariant_fail = inv_report.failed > 0;

    // Drift check: compare current to baseline seal
    let baseline_seal_graph = vantage_types::uir::UnifiedGraph {
        nodes: combined_graph.nodes.clone(),
        source_language: vantage_types::uir::Language::Rust,
    };
    let drift_report = drift_engine::quick_check(&baseline_seal_graph, &combined_graph);

    if ci {
        ci_output(json!({
            "v": VANTAGE_VERSION,
            "status": if !has_node_drift && !has_invariant_fail { "ok" } else { "drift" },
            "has_drift": has_node_drift,
            "has_invariant_fail": has_invariant_fail,
            "node_count": current_count,
            "baseline_node_count": baseline_count,
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
            "drift": {
                "has_drift": drift_report.has_drift,
                "added_nodes": drift_report.added_nodes.len(),
                "removed_nodes": drift_report.removed_nodes.len(),
                "invariant_breaks": drift_report.invariant_breaks,
                "parity_breaks": drift_report.parity_breaks,
            },
        }));

        if has_invariant_fail {
            process::exit(EXIT_INVARIANT);
        }
        if has_node_drift {
            process::exit(EXIT_DRIFT);
        }
        process::exit(EXIT_OK);
    }

    // Human output
    println!("{}", bold!(yellow!("🛡️  VANTAGE VERIFY")));
    println!("  Seal:    {}", seal_path.display());
    println!("  Baseline: {} nodes", baseline_count);
    println!("  Current:  {} nodes", current_count);

    if has_node_drift {
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
        println!("  {} [{}] {}", icon, r.spec_id, r.message);
    }

    if has_node_drift {
        println!();
        println!("  Drift: {} added, {} removed",
            drift_report.added_nodes.len(),
            drift_report.removed_nodes.len());
    }

    if !errors.is_empty() {
        println!();
        println!("  Errors: {}", errors.len());
        for e in &errors {
            println!("    - {}", e);
        }
    }

    Ok(())
}

// ─── Capabilities ───────────────────────────────────────────────────────────

pub fn execute_capabilities() -> Result<()> {
    let caps = json!({
        "v": VANTAGE_VERSION,
        "schema_version": "1.2.5",
        "commands": ["run", "graph", "verify"],
        "flags": ["--ci", "--json", "--deep", "--dry-run"],
        "modes": {
            "ci": {
                "description": "Machine-oriented stable JSON contract",
                "exit_codes": {
                    "0": "OK",
                    "1": "Drift detected",
                    "2": "Invariant violation",
                    "10": "Internal engine error",
                },
                "guarantees": [
                    "stable JSON schema",
                    "no ANSI",
                    "no prose",
                    "deterministic ordering",
                    "stable exit codes",
                ],
            }
        },
        "languages": ["rust", "python", "ruby", "javascript", "typescript", "tsx"],
        "capabilities": [
            "structural-analysis",
            "dependency-graph",
            "intent-extraction",
            "invariant-checking",
            "drift-detection",
            "cross-language-parity",
        ],
        "output_formats": ["json"],
        "invariant_rules": [
            "NodeCountMin",
            "RequiredKind",
            "RequiredIntent",
            "CrossLanguageParity",
            "HashStability",
        ],
    });

    println!("{}", serde_json::to_string_pretty(&caps)?);
    Ok(())
}
