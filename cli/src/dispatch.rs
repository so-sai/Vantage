//! # Structural Dispatch Module (v1.2.4)
//!
//! Handles Forensic Structural Intents with optional enforcement pipeline.
//! Pipeline: signal → claim → invariant → decision.

use anyhow::{Context, Result};
use colored::*;
use ignore::WalkBuilder;
use serde_json::json;
use std::path::PathBuf;
use vantage_core::cognition::{ClaimType, Decision, InvariantRule, Pipeline};
use vantage_core::parser::Language;
use vantage_core::FailureReason;
use vantage_core::VANTAGE_VERSION;

fn print_json_error(reason: FailureReason, message: &str, file: Option<&str>) {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "v": VANTAGE_VERSION,
            "status": "error",
            "reason": reason,
            "message": message,
            "file": file,
        }))
        .unwrap_or_default()
    );
}

/// Analyze a single file and output structural signals
pub fn execute_verify_file(path: PathBuf, use_json: bool, enforce: bool) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let lang = match Language::from_extension(ext) {
        Some(l) => l,
        None => {
            if use_json {
                print_json_error(
                    FailureReason::UnsupportedLanguage,
                    &format!("Unsupported file extension: {}", ext),
                    Some(&path_str),
                );
            } else {
                eprintln!("Error: Unsupported file extension: {}", ext);
            }
            std::process::exit(1);
        }
    };

    if enforce {
        return execute_enforce(path, lang, use_json);
    }

    let mut pipeline = match Pipeline::new(lang) {
        Ok(p) => p,
        Err(e) => {
            if use_json {
                print_json_error(FailureReason::InternalError, &e, Some(&path_str));
            } else {
                eprintln!("Error: {}", e);
            }
            std::process::exit(1);
        }
    };

    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            if use_json {
                print_json_error(
                    FailureReason::FileReadError,
                    &e.to_string(),
                    Some(&path_str),
                );
            } else {
                eprintln!("Error: Failed to read file: {}", e);
            }
            std::process::exit(1);
        }
    };

    let result = pipeline.run(&source, &path_str);

    if use_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "v": VANTAGE_VERSION,
                "status": "ok",
                "file": result.file,
                "language": ext,
                "signals": result.signals.len(),
                "claims": result.claims.len(),
                "verdicts": result.verdicts,
                "final_decision": format!("{:?}", result.final_decision).to_lowercase(),
                "duration_ms": result.duration_ms,
            }))?
        );
    } else {
        println!("⚓ Anchor: {}", path_str.bright_blue());
        println!("Signals: {}", result.signals.len().to_string().yellow());
        for sig in &result.signals {
            println!(
                "  └─ [{:?}] {} :: {}",
                sig.symbol_kind,
                sig.symbol_id.green(),
                &sig.structural_hash[..8].cyan()
            );
        }
        println!("Claims: {}", result.claims.len().to_string().yellow());
        for claim in &result.claims {
            println!(
                "  └─ [{:?}] (confidence: {:.0}%)",
                claim.claim_type,
                claim.confidence * 100.0
            );
        }
        println!("Verdicts: {}", result.verdicts.len().to_string().yellow());
        for verdict in &result.verdicts {
            let symbol = match verdict.decision {
                Decision::Allow => "✅",
                Decision::Warn => "⚠️",
                Decision::Reject => "🚫",
            };
            println!(
                "  {} [{:?}] {} - {}",
                symbol, verdict.decision, verdict.claim_id, verdict.reason
            );
        }
        println!("⏱️  {}ms", result.duration_ms.to_string().bright_black());
    }

    Ok(())
}

fn execute_enforce(path: PathBuf, lang: Language, use_json: bool) -> Result<()> {
    let path_str = path.to_string_lossy().to_string();

    let mut pipeline = Pipeline::new(lang).map_err(|e| anyhow::anyhow!(e))?;

    // Add enforcement rules
    pipeline.engine.add_rule(Box::new(InvariantRule {
        name: "forbid_template_interpolation".to_string(),
        claim_type: ClaimType::TemplateInterpolation,
        decision: Decision::Warn,
        reason: "Template interpolation detected - verify input sanitization".to_string(),
    }));

    let source = std::fs::read_to_string(&path).context("Failed to read file")?;
    let result = pipeline.run(&source, &path_str);

    if use_json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "v": VANTAGE_VERSION,
                "status": "ok",
                "mode": "enforce",
                "file": result.file,
                "language": format!("{:?}", lang).to_lowercase(),
                "signals": result.signals.len(),
                "claims": result.claims.len(),
                "verdicts": result.verdicts,
                "final_decision": format!("{:?}", result.final_decision).to_lowercase(),
                "duration_ms": result.duration_ms,
            }))?
        );
    } else {
        println!(
            "{}",
            "🛡️  VANTAGE ENFORCEMENT PIPELINE (v1.2.4)"
                .bright_yellow()
                .bold()
        );
        println!("📁 File: {}", path_str.bright_blue());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );

        println!(
            "\n📡 Signals: {}",
            result.signals.len().to_string().yellow()
        );
        for sig in &result.signals {
            println!(
                "  └─ [{:?}] {} :: {}",
                sig.symbol_kind,
                sig.symbol_id.green(),
                &sig.structural_hash[..8].cyan()
            );
        }

        println!("\n🧠 Claims: {}", result.claims.len().to_string().yellow());
        for claim in &result.claims {
            println!(
                "  └─ [{:?}] (confidence: {:.0}%)",
                claim.claim_type,
                claim.confidence * 100.0
            );
        }

        println!("\n⚖️  Verdicts:");
        for verdict in &result.verdicts {
            let symbol = match verdict.decision {
                Decision::Allow => "✅".green(),
                Decision::Warn => "⚠️ ".yellow(),
                Decision::Reject => "🚫".red(),
            };
            println!("  {} [{:?}] {}", symbol, verdict.decision, verdict.reason);
        }

        let final_color = match result.final_decision {
            Decision::Allow => "ALLOW".green().bold(),
            Decision::Warn => "WARN".yellow().bold(),
            Decision::Reject => "REJECT".red().bold(),
        };
        println!("\n🏁 Final Decision: {}", final_color);

        if result.final_decision == Decision::Reject {
            std::process::exit(1);
        }
    }

    Ok(())
}

/// Execute Seal intent to finalize structural baseline
pub fn execute_seal(path: PathBuf) -> Result<()> {
    println!(
        "{}",
        "🛡️  VANTAGE STRUCTURAL SEALING (v1.2.4)"
            .bright_yellow()
            .bold()
    );
    println!("📁 Target: {}", path.display().to_string().bright_blue());
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
    );

    let mut map = Vec::new();
    let walker = WalkBuilder::new(&path)
        .hidden(false)
        .git_ignore(true)
        .build();

    for entry in walker.flatten() {
        let p = entry.path();
        if p.is_file() {
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "rs" || ext == "py" {
                let lang = match Language::from_extension(ext) {
                    Some(l) => l,
                    None => continue,
                };
                let mut pipe = match Pipeline::new(lang) {
                    Ok(p) => p,
                    Err(_) => continue,
                };
                let source = match std::fs::read_to_string(p) {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let rel_path = p
                    .strip_prefix(&path)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .to_string();
                let result = pipe.run(&source, &rel_path);
                for sig in result.signals {
                    map.push(json!({
                        "f": rel_path,
                        "s": sig.symbol_id,
                        "h": sig.structural_hash,
                        "n": sig.normalized_hash,
                    }));
                }
            }
        }
    }

    let seal_data = json!({
        "v": "1.2.4",
        "ts": chrono::Utc::now().to_rfc3339(),
        "map": map,
    });

    let seal_path = path.join("VANTAGE.SEAL");
    std::fs::write(&seal_path, serde_json::to_string_pretty(&seal_data)?)?;

    println!("✅ SUCCESS: Forensic baseline established.");
    println!(
        "📦 Map contains {} structural signals.",
        map.len().to_string().yellow()
    );
    println!(
        "📝 Written to: {}",
        seal_path.display().to_string().bright_green()
    );

    Ok(())
}

/// Purge local forensic artifacts
pub fn execute_purge(force: bool) -> Result<()> {
    println!("{}", "🧹 VANTAGE PURGE".bright_red().bold());

    if !force {
        anyhow::bail!("Purge requires {} flag for safety.", "--force".yellow());
    }

    let seal_path = PathBuf::from("VANTAGE.SEAL");
    if seal_path.exists() {
        std::fs::remove_file(&seal_path)?;
        println!(
            "🗑️ Removed: {}",
            seal_path.display().to_string().bright_green()
        );
    } else {
        println!("ℹ️ No forensic artifacts found to purge.");
    }

    println!("✨ Workspace is now clean.");
    Ok(())
}

/// Diff current file against VANTAGE.SEAL baseline
pub fn execute_diff(path: PathBuf, seal_path: PathBuf, use_json: bool) -> Result<()> {
    use vantage_core::parser::Language;
    use vantage_core::DriftReport;

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let lang = Language::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", ext))?;

    // Parse current file
    let source = std::fs::read_to_string(&path).context("Failed to read file")?;
    let mut pipeline = Pipeline::new(lang).map_err(|e| anyhow::anyhow!(e))?;
    let current_result = pipeline.run(&source, &path.to_string_lossy());

    // Parse baseline seal
    let seal_data = std::fs::read_to_string(&seal_path)
        .context("VANTAGE.SEAL not found. Run 'vantage seal' first.")?;
    let seal: serde_json::Value = serde_json::from_str(&seal_data)?;

    // Extract baseline signals for this file from seal
    // Match by normalized_hash (rename-invariant) instead of symbol_id
    let file_rel = path.file_name().unwrap_or_default().to_string_lossy();

    // Build baseline signals from seal data (minimal CognitiveSignal reconstruction)
    let baseline_signals: Vec<vantage_core::CognitiveSignal> = seal["map"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .filter(|entry| entry["f"].as_str().is_some_and(|f| f.contains(&*file_rel)))
        .filter_map(|entry| {
            let sym_id = entry["s"].as_str()?.to_string();
            let struct_hash = entry["h"].as_str()?.to_string();
            let norm_hash = entry["n"].as_str()?.to_string();
            Some(vantage_core::CognitiveSignal {
                uuid: String::new(),
                symbol_id: sym_id,
                parent: None,
                symbol_kind: vantage_core::SymbolKind::Other("sealed".to_string()),
                language: String::new(),
                structural_hash: struct_hash,
                semantic_hash: String::new(),
                normalized_hash: norm_hash,
                signature: None,
                location: vantage_core::SourceLocation {
                    file: String::new(),
                    start_line: 0,
                    start_col: 0,
                    end_line: 0,
                    end_col: 0,
                    byte_start: 0,
                    byte_end: 0,
                },
                metadata: std::collections::HashMap::new(),
                origin: vantage_core::Origin {
                    parser: String::new(),
                    version: String::new(),
                },
                confidence: 1.0,
            })
        })
        .collect();

    // Build current signals indexed by normalized_hash for rename-invariant matching
    let current_by_norm: std::collections::HashMap<String, &vantage_core::CognitiveSignal> =
        current_result
            .signals
            .iter()
            .map(|s| (s.normalized_hash.clone(), s))
            .collect();

    // For each baseline signal: if normalized_hash matches current, update structural_hash
    // to baseline value so DriftReport can detect drift. If no match, keep baseline as-is
    // so DriftReport detects it as "removed".
    let aligned_baseline: Vec<vantage_core::CognitiveSignal> = baseline_signals
        .iter()
        .map(|baseline| {
            let mut b = baseline.clone();
            if let Some(current) = current_by_norm.get(&baseline.normalized_hash) {
                b.structural_hash = baseline.structural_hash.clone();
                b.location = current.location.clone();
                b.symbol_id = current.symbol_id.clone();
            }
            b
        })
        .collect();

    let report = DriftReport::compare(&aligned_baseline, &current_result.signals);

    if use_json {
        let mut output = serde_json::to_value(&report)?;
        if let Some(obj) = output.as_object_mut() {
            obj.insert("v".to_string(), json!(VANTAGE_VERSION));
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "{}",
            "📊 VANTAGE DRIFT REPORT (v1.2.4)".bright_yellow().bold()
        );
        println!("📁 File: {}", path.to_string_lossy().bright_blue());
        println!(
            "📦 Baseline: {}",
            seal_path.display().to_string().bright_black()
        );
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );

        println!("\n📊 Summary:");
        println!(
            "  Total symbols: {}",
            report.total_symbols.to_string().yellow()
        );
        println!("  ✅ Unchanged:  {}", report.unchanged.to_string().green());
        println!(
            "  🔶 Struct chg: {}",
            report.structural_changes.to_string().yellow()
        );
        println!(
            "  🔴 Semantic:   {}",
            report.semantic_changes.to_string().red()
        );
        println!("  ➕ Added:      {}", report.added.to_string().cyan());
        println!("  ➖ Removed:    {}", report.removed.to_string().magenta());

        println!("\n📋 Details:");
        for item in &report.items {
            let (symbol, desc) = match item.status {
                vantage_core::DriftStatus::Unchanged => ("✅", "unchanged".green()),
                vantage_core::DriftStatus::StructuralChange => ("🔶", "structural change".yellow()),
                vantage_core::DriftStatus::SemanticChange => ("🔴", "semantic change".red()),
                vantage_core::DriftStatus::Added => ("➕", "added".cyan()),
                vantage_core::DriftStatus::Removed => ("➖", "removed".magenta()),
            };
            println!(
                "  {} {} @ {} — {}",
                symbol,
                item.symbol_id.green(),
                item.location.bright_black(),
                desc
            );
        }

        let has_changes = report.structural_changes > 0
            || report.semantic_changes > 0
            || report.added > 0
            || report.removed > 0;
        if has_changes {
            println!("\n{}", "⚠️  DRIFT DETECTED".red().bold());
        } else {
            println!("\n{}", "✅ NO DRIFT".green().bold());
        }
    }

    Ok(())
}

/// Extract dependency graph from source file
pub fn execute_graph(path: PathBuf, use_json: bool) -> Result<()> {
    use vantage_core::parser::Language;

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let lang = Language::from_extension(ext)
        .ok_or_else(|| anyhow::anyhow!("Unsupported file extension: {}", ext))?;

    let mut pipeline = Pipeline::new(lang).map_err(|e| anyhow::anyhow!(e))?;
    let source = std::fs::read_to_string(&path).context("Failed to read file")?;

    let (signals, graph) = pipeline
        .parser
        .parse_with_graph(&source, &path.to_string_lossy());

    if use_json {
        let sorted_edges: Vec<_> = graph.sorted_edges().into_iter().cloned().collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "v": VANTAGE_VERSION,
                "status": "ok",
                "file": path.to_string_lossy(),
                "signals": signals.len(),
                "graph": {
                    "nodes": graph.nodes.len(),
                    "edges": sorted_edges,
                }
            }))?
        );
    } else {
        println!(
            "{}",
            "🕸️  VANTAGE SYMBOL GRAPH (v1.2.4)".bright_yellow().bold()
        );
        println!("📁 File: {}", path.to_string_lossy().bright_blue());
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );

        println!("\n📡 Signals: {}", signals.len().to_string().yellow());
        for sig in &signals {
            println!("  └─ [{:?}] {}", sig.symbol_kind, sig.symbol_id.green());
        }

        println!("\n🔗 Edges: {}", graph.edges.len().to_string().yellow());
        for edge in &graph.edges {
            let arrow = match edge.edge_type {
                vantage_core::EdgeType::Calls => "→ calls →",
                vantage_core::EdgeType::Imports => "→ imports →",
                vantage_core::EdgeType::Uses => "→ uses →",
            };
            println!(
                "  {} {} {}",
                edge.from.cyan(),
                arrow.bright_black(),
                edge.to.cyan()
            );
        }

        if graph.edges.is_empty() {
            println!("  (no call/import edges detected)");
        }

        // Show impact radius for each signal
        if !signals.is_empty() && !graph.edges.is_empty() {
            println!("\n💥 Impact Radius:");
            for sig in &signals {
                let impacted = graph.impact_radius(&sig.symbol_id);
                if !impacted.is_empty() {
                    println!(
                        "  {} ← {}",
                        sig.symbol_id.green(),
                        impacted
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                            .yellow()
                    );
                }
            }
        }
    }

    Ok(())
}
