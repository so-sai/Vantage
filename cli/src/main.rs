//! # Vantage CLI (v1.2.5)
//!
//! Three commands + CI mode for machine-oriented invocation.
//!   run     — parse → normalize → seal
//!   graph   — extract + output unified dependency graph
//!   verify  — seal integrity + drift + invariants
//!   --ci    — stable JSON contract for automation/agents

mod dispatch;
mod kit_integration;
mod term;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "kit-vantage")]
#[command(version = "1.2.5")]
#[command(about = "Vantage Structural Sensor — multi-language architectural intent graph", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse source → normalize → seal. Default action for structural extraction.
    Run {
        /// Path to file or directory
        path: PathBuf,
        /// Machine-oriented output: stable JSON, no ANSI, deterministic exit codes
        #[arg(long)]
        ci: bool,
        /// Skip seal creation (parse + normalize only)
        #[arg(long)]
        dry_run: bool,
    },
    /// Extract and output the unified dependency graph (JSON).
    Graph {
        /// Path to file or directory
        path: PathBuf,
        /// Machine-oriented output: stable JSON, no ANSI, deterministic exit codes
        #[arg(long)]
        ci: bool,
    },
    /// Verify seal integrity, run invariants, detect drift.
    Verify {
        /// Path to file or directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Machine-oriented output: stable JSON, no ANSI, deterministic exit codes
        #[arg(long)]
        ci: bool,
        /// Deep verification with baseline comparison
        #[arg(long, short = 'd')]
        deep: bool,
    },
    /// List all capabilities this version supports (machine-readable JSON).
    Capabilities,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run { path, ci, dry_run } => {
            dispatch::execute_run(path, ci, dry_run)
        }
        Commands::Graph { path, ci } => {
            dispatch::execute_graph(path, ci)
        }
        Commands::Verify { path, ci, deep } => {
            dispatch::execute_verify(path, ci, deep)
        }
        Commands::Capabilities => {
            dispatch::execute_capabilities()
        }
    }
}
