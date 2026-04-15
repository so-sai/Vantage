//! # Vantage CLI (v1.2.4)
//!
//! Pure forensic structural sensor with execution pipeline.
//! Supports signal → claim → invariant → decision enforcement.

mod dispatch;
mod term;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vantage")]
#[command(version = "1.2.4-ULTRA-LEAN")]
#[command(about = "Vantage Structural Sensor - [ZERO-LAG] CPU-bound forensic extraction", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a single file and output structural signals
    Verify {
        /// Path to the source file
        path: PathBuf,
        /// Output results in JSON format
        #[arg(long)]
        json: bool,
        /// Run enforcement pipeline
        #[arg(long)]
        enforce: bool,
    },
    /// Diff current file against VANTAGE.SEAL baseline [EXPERIMENTAL]
    Diff {
        /// Path to the source file
        path: PathBuf,
        /// Path to the seal file (defaults to VANTAGE.SEAL)
        #[arg(long, default_value = "VANTAGE.SEAL")]
        seal: PathBuf,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Extract dependency graph from source file [EXPERIMENTAL]
    Graph {
        /// Path to the source file
        path: PathBuf,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
    },
    /// Seal current directory state [EXPERIMENTAL]
    Seal {
        /// Path to the directory or file
        path: PathBuf,
    },
    /// Purge local forensic artifacts [EXPERIMENTAL]
    Purge {
        /// Force removal without confirmation
        #[arg(long)]
        message: Option<String>,
        #[arg(long)]
        force: bool,
    },
    /// Benchmark incremental performance on synthetic loads
    Bench {
        /// Number of iterations for timing
        #[arg(long, default_value = "10")]
        iterations: usize,
    },
    /// Introspect Vantage capabilities [EXPERIMENTAL]
    Introspect {
        /// List all capabilities
        #[arg(long)]
        list: bool,
        /// Show detailed info for a specific capability
        #[arg(long)]
        capability: Option<String>,
        /// Output in JSON format
        #[arg(long)]
        json: bool,
        /// Show system envelope (limits and invariants)
        #[arg(long)]
        envelope: bool,
        /// Show performance limits
        #[arg(long)]
        limits: bool,
    },
}

fn main() -> Result<()> {
    // Elite Observability: Initialize Tracing with ERROR as default leaf
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("error")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Verify {
            path,
            json,
            enforce,
        } => dispatch::execute_verify_file(path, json, enforce),
        Commands::Diff { path, seal, json } => dispatch::execute_diff(path, seal, json),
        Commands::Graph { path, json } => dispatch::execute_graph(path, json),
        Commands::Seal { path } => dispatch::execute_seal(path),
        Commands::Purge { message: _, force } => dispatch::execute_purge(force),
        Commands::Bench { iterations } => dispatch::execute_bench(iterations),
        Commands::Introspect {
            list,
            capability,
            json,
            envelope,
            limits,
        } => dispatch::execute_introspect(list, capability, json, envelope, limits),
    };

    if let Err(e) = result {
        tracing::error!("Forensic failure: {:?}", e);
        std::process::exit(1);
    }

    Ok(())
}
