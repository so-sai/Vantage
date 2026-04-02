//! # Vantage CLI (v1.2.4)
//!
//! Pure forensic structural sensor with execution pipeline.
//! Supports signal → claim → invariant → decision enforcement.

mod dispatch;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "vantage")]
#[command(about = "Vantage Structural Sensor - Plugin for kit", long_about = None)]
#[command(version = "1.2.4")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze a file and output structural signals (L2 Lens)
    Verify {
        /// Path to the source file (.rs, .py)
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Run enforcement pipeline (signal → claim → invariant → decision)
        #[arg(long)]
        enforce: bool,
    },
    /// Diff current file against VANTAGE.SEAL baseline
    Diff {
        /// Path to the source file to compare
        path: PathBuf,

        /// Path to VANTAGE.SEAL (default: ./VANTAGE.SEAL)
        #[arg(long, default_value = "VANTAGE.SEAL")]
        seal: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Extract dependency graph from source file
    Graph {
        /// Path to the source file (.rs, .py)
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Establish a forensic structural baseline for the project
    Seal {
        /// Directory to seal (default: current dir)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Clear local artifacts (Clean mode)
    Purge {
        /// Force removal without confirmation
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Verify {
            path,
            json,
            enforce,
        } => dispatch::execute_verify_file(path, json, enforce),
        Commands::Diff { path, seal, json } => dispatch::execute_diff(path, seal, json),
        Commands::Graph { path, json } => dispatch::execute_graph(path, json),
        Commands::Seal { path } => dispatch::execute_seal(path),
        Commands::Purge { force } => dispatch::execute_purge(force),
    }
}
