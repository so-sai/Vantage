//! # Vantage CLI - Reference Adapter
//!
//! This is the reference implementation for all Vantage adapters.
//! It emits `CognitiveIntent` and dispatches to Vantage Core.
//!
//! ## Usage
//! ```bash
//! vantage lens README.md
//! vantage export design.md --mode visual
//! vantage drift ./project
//! ```
//!
//! ## Architecture
//! ```
//! CLI args → CognitiveIntent → Vantage Core → Output
//! ```
//!
//! All IDE adapters (VS Code, Open Code, Antigravity) should follow
//! the same pattern: UI event → CognitiveIntent → MCP/Core.

mod dispatch;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use vantage_core::intent::{
    CognitiveIntent, IntentBuilder, IntentKind, IntentSource, IntentTarget,
};

/// Vantage - Cognitive Artifact Factory
///
/// Transform documents into semantic artifacts with zero-token overhead.
#[derive(Parser)]
#[command(name = "vantage")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze document structure (semantic lens)
    Lens {
        /// Path to the document to analyze
        file: PathBuf,
    },

    /// Export document to PDF
    Export {
        /// Path to the document to export
        file: PathBuf,

        /// Export mode: semantic, visual, or audit
        #[arg(short, long, default_value = "semantic")]
        mode: String,

        /// Output file path (defaults to input with .pdf extension)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Check drift between blueprint and code
    Drift {
        /// Path to the workspace/project directory
        #[arg(default_value = ".")]
        workspace: PathBuf,

        /// Path to the blueprint document
        #[arg(short, long, default_value = "DESIGN.md")]
        blueprint: String,
    },

    /// Check core system status
    Status,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Lens { file } => {
            let intent = build_lens_intent(&file)?;
            dispatch::execute_intent(intent)
        }
        Commands::Export { file, mode, output } => {
            let intent = build_export_intent(&file, &mode)?;
            dispatch::execute_export(intent, output)
        }
        Commands::Drift {
            workspace,
            blueprint,
        } => {
            let intent = build_drift_intent(&workspace)?;
            dispatch::execute_drift(intent, &blueprint)
        }
        Commands::Status => {
            println!("🟢 Vantage Core: {}", vantage_core::get_agent_status());
            Ok(())
        }
    }
}

/// Build a Lens intent from CLI args
fn build_lens_intent(file: &PathBuf) -> Result<CognitiveIntent> {
    IntentBuilder::new()
        .kind(IntentKind::Lens)
        .source(IntentSource::Cli)
        .target(IntentTarget::file(file))
        .build()
        .context("Failed to build Lens intent")
}

/// Build an Export intent from CLI args
fn build_export_intent(file: &PathBuf, mode: &str) -> Result<CognitiveIntent> {
    IntentBuilder::new()
        .kind(IntentKind::Export)
        .source(IntentSource::Cli)
        .target(IntentTarget::file(file))
        .param("mode", mode)
        .build()
        .context("Failed to build Export intent")
}

/// Build a Drift intent from CLI args
fn build_drift_intent(workspace: &PathBuf) -> Result<CognitiveIntent> {
    IntentBuilder::new()
        .kind(IntentKind::Drift)
        .source(IntentSource::Cli)
        .target(IntentTarget::workspace(workspace))
        .build()
        .context("Failed to build Drift intent")
}
