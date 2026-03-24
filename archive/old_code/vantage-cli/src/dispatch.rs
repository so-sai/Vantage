//! # Intent Dispatch Module
//!
//! This module handles the execution of CognitiveIntent by calling Vantage Core.
//!
//! ## Architecture Note
//! In a full MCP setup, this would dispatch to the MCP server.
//! For CLI, we call Core directly for simplicity and speed.

use anyhow::{Context, Result};
use std::path::PathBuf;
use vantage_core::intent::{CognitiveIntent, IntentKind, IntentTarget};
use vantage_core::{
    document_lens, export_to_pdf, check_drift,
    ExportConfig, ExportMode, DriftConfig,
};

/// Execute a CognitiveIntent (Lens only)
pub fn execute_intent(intent: CognitiveIntent) -> Result<()> {
    match intent.intent {
        IntentKind::Lens => execute_lens(&intent),
        _ => anyhow::bail!("Unexpected intent type for execute_intent"),
    }
}

/// Execute Lens intent
fn execute_lens(intent: &CognitiveIntent) -> Result<()> {
    let path = match &intent.target {
        IntentTarget::File { path, .. } => path,
        IntentTarget::Workspace { .. } => {
            anyhow::bail!("Lens requires a file target")
        }
    };

    let path_str = path.to_string_lossy();
    let doc = document_lens(&path_str)
        .context(format!("Failed to analyze document: {}", path_str))?;

    // Output semantic analysis
    println!("📄 Document Analysis: {}", path_str);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📊 Type: {}", doc.metadata.source_type);
    println!("📝 Words: {}", doc.metadata.word_count);
    
    if !doc.headers.is_empty() {
        println!("\n📑 Structure ({} headers):", doc.headers.len());
        for (i, header) in doc.headers.iter().take(10).enumerate() {
            println!("   {}. {}", i + 1, header);
        }
        if doc.headers.len() > 10 {
            println!("   ... and {} more", doc.headers.len() - 10);
        }
    }

    if !doc.sheets.is_empty() {
        println!("\n📊 Sheets ({}):", doc.sheets.len());
        for sheet in &doc.sheets {
            println!("   • {} ({} rows)", sheet.name, sheet.data.len());
        }
    }

    Ok(())
}

/// Execute Export intent with output path
pub fn execute_export(intent: CognitiveIntent, output: Option<PathBuf>) -> Result<()> {
    let path = match &intent.target {
        IntentTarget::File { path, .. } => path,
        IntentTarget::Workspace { .. } => {
            anyhow::bail!("Export requires a file target")
        }
    };

    // Parse mode from params
    let mode_str = intent
        .params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("semantic");

    let mode = match mode_str {
        "visual" => ExportMode::Visual,
        "audit" => ExportMode::Audit,
        _ => ExportMode::Semantic,
    };

    let config = ExportConfig {
        mode: mode.clone(),
        include_toc: true,
        include_metadata: true,
    };

    // Load document
    let path_str = path.to_string_lossy();
    let doc = document_lens(&path_str)
        .context(format!("Failed to read document: {}", path_str))?;

    // Export to PDF
    let pdf_bytes = export_to_pdf(&doc, config)
        .context("Failed to generate PDF")?;

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut out = path.clone();
        out.set_extension("pdf");
        out
    });

    // Write PDF
    std::fs::write(&output_path, &pdf_bytes)
        .context(format!("Failed to write PDF to: {}", output_path.display()))?;

    println!("✅ Exported: {} → {}", path_str, output_path.display());
    println!("   Mode: {:?}", mode);
    println!("   Size: {} bytes", pdf_bytes.len());

    Ok(())
}

/// Execute Drift intent with blueprint path
pub fn execute_drift(intent: CognitiveIntent, blueprint: &str) -> Result<()> {
    let workspace = match &intent.target {
        IntentTarget::Workspace { root } => root,
        IntentTarget::File { .. } => {
            anyhow::bail!("Drift requires a workspace target")
        }
    };

    let config = DriftConfig {
        blueprint_path: blueprint.to_string(),
        code_path: workspace.to_string_lossy().to_string(),
        section_header: None,
        ignore_patterns: Some(vec![
            "test_*".to_string(),
            "mock_*".to_string(),
        ]),
    };

    let report = check_drift(config)
        .context("Failed to check drift")?;

    // Output drift report
    println!("🔍 Drift Report: {}", workspace.display());
    println!("   Blueprint: {}", blueprint);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Aligned:       {}", report.aligned.len());
    println!("⚠️  Undocumented:  {}", report.undocumented.len());
    println!("🚧 Unimplemented: {}", report.unimplemented.len());

    if !report.undocumented.is_empty() {
        println!("\n⚠️  Undocumented functions (drift):");
        for func in report.undocumented.iter().take(5) {
            println!("   • {}", func);
        }
        if report.undocumented.len() > 5 {
            println!("   ... and {} more", report.undocumented.len() - 5);
        }
    }

    if !report.unimplemented.is_empty() {
        println!("\n🚧 Unimplemented spec items:");
        for item in report.unimplemented.iter().take(5) {
            println!("   • {}", item);
        }
        if report.unimplemented.len() > 5 {
            println!("   ... and {} more", report.unimplemented.len() - 5);
        }
    }

    Ok(())
}
