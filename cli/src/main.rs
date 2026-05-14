//! # Vantage CLI (v1.2.5)
//!
//! Three commands, no clutter.
//!   run     — parse → normalize → seal
//!   graph   — extract + output unified dependency graph
//!   verify  — seal integrity + drift detection

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
        /// Output results in JSON format
        #[arg(long)]
        json: bool,
        /// Skip seal creation (parse + normalize only)
        #[arg(long)]
        dry_run: bool,
    },
    /// Extract and output the unified dependency graph (JSON).
    Graph {
        /// Path to file or directory
        path: PathBuf,
    },
    /// Verify seal integrity and detect structural drift.
    Verify {
        /// Path to file or directory (defaults to current directory)
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Output in JSON format
        #[arg(long, short = 'j')]
        json: bool,
        /// Deep verification with baseline comparison
        #[arg(long, short = 'd')]
        deep: bool,
    },
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
        Commands::Run { path, json, dry_run } => {
            dispatch::execute_run(path, json, dry_run)
        }
        Commands::Graph { path } => {
            dispatch::execute_graph(path)
        }
        Commands::Verify { path, json, deep } => {
            dispatch::execute_verify(path, json, deep)
        }
    }
}
