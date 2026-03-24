// Vantage Core Entry Point
// Stateless Epistemic Enforcement System

use std::env;
use vantage_core::cognition::*;

fn main() {
    println!("🛡️ VANTAGE CORE - STATELESS EPISTEMIC ENFORCEMENT");
    println!("📋 For CLI operations, use vantage-verify binary");
    println!("📋 Status: RC 1.0 - LOCKED AND VERIFIED");

    // Check if any arguments provided
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        // Check for verify command
        if args[1] == "verify" && args.len() == 3 {
            let file_path = &args[2];
            run_verification(file_path);
        } else {
            print_usage();
        }
    } else {
        print_usage();
    }
}

fn print_usage() {
    println!("\nUsage:");
    println!("  vantage-core                    - Show status");
    println!("  vantage-core verify <file>      - Verify epistemic claims");
    println!("\nExamples:");
    println!("  vantage-core");
    println!("  vantage-core verify src/main.rs");
}

fn run_verification(file_path: &str) {
    println!("🔍 Verifying epistemic signals in: {}", file_path);

    let path = std::path::Path::new(file_path);
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let source = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("❌ Error reading file '{}': {}", file_path, e);
            std::process::exit(1);
        }
    };

    let mut parser = match extension {
        "rs" => EpistemicParser::new_rust_parser().unwrap(),
        "py" => EpistemicParser::new_python_parser().unwrap(),
        _ => {
            eprintln!("❌ Unsupported file extension: .{}", extension);
            std::process::exit(1);
        }
    };

    let signals = parser.parse_signals(&source);

    if signals.is_empty() {
        println!("✅ No epistemic signals found");
        println!("📊 Report: 0 signals | 0 drifts | CLEAN");
    } else {
        println!("🔍 Found {} epistemic signal(s)", signals.len());

        for (i, signal) in signals.iter().enumerate() {
            println!("\n  {}. UUID: {}", i + 1, signal.uuid);
            println!("     Symbol: {}", signal.symbol_id);
            println!("     Language: {}", signal.language);
            println!("     Structural Hash: {}", signal.structural_hash);
            println!("     Semantic Hash: {}", signal.semantic_hash);
        }

        println!("\n📋 Verification Complete");
        println!(
            "📊 Report: {} signals | READY FOR MANIFEST COMPARISON",
            signals.len()
        );
    }
}
