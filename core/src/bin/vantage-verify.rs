use std::env;
use std::path::Path;
use vantage_core::parser::{get_parser, Language};
use vantage_core::*;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_json = false;

    // Very basic arg parsing for v1.2.3
    let mut file_path = None;
    for arg in args.iter().skip(1) {
        if arg == "--json" {
            use_json = true;
        } else if file_path.is_none() {
            file_path = Some(arg);
        }
    }

    let file_path = match file_path {
        Some(p) => p,
        None => {
            if use_json {
                println!("{}", serde_json::json!({
                    "status": "error",
                    "reason": "Missing file path argument"
                }));
            } else {
                eprintln!("Usage: vantage-verify <file-path> [--json]");
            }
            std::process::exit(1);
        }
    };

    let ext = Path::new(file_path)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let lang = match Language::from_extension(ext) {
        Some(l) => l,
        None => {
            if use_json {
                println!("{}", serde_json::json!({
                    "status": "error",
                    "reason": format!("Unsupported file extension: {}", ext)
                }));
            } else {
                eprintln!("❌ Unsupported file extension: {}", ext);
            }
            std::process::exit(1);
        }
    };

    let mut parser = get_parser(lang);
    
    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            if use_json {
                println!("{}", serde_json::json!({
                    "status": "error",
                    "reason": format!("Error reading file: {}", e)
                }));
            } else {
                eprintln!("❌ Error reading file: {}", e);
            }
            std::process::exit(1);
        }
    };

    let signals = parser.parse_signals(&content, file_path);

    if use_json {
        let reason = if signals.is_empty() {
             Some("no_anchor_found".to_string())
        } else {
            None
        };

        let result = serde_json::json!({
            "status": "ok",
            "language": format!("{:?}", lang).to_lowercase(),
            "signals": signals.iter().map(|s| {
                serde_json::json!({
                    "id": s.uuid,
                    "name": s.symbol_id,
                    "type": format!("{:?}", s.symbol_kind).to_lowercase(),
                    "structural_hash": s.structural_hash,
                    "normalized_hash": s.normalized_hash,
                    "signature": s.signature,
                    "location": {
                        "file": file_path,
                        "line": 1 // Placeholder for now
                    }
                })
            }).collect::<Vec<_>>(),
            "reason": reason
        });
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("🛡️  VANTAGE STRUCTURAL VERIFY (v1.2.3)  🛡️");
        println!("📁 File: {}", file_path);
        
        if signals.is_empty() {
            println!("✅ No structural anchors found (@epistemic tag missing)");
            return;
        }

        println!("🔍 Found {} structural signal(s)", signals.len());
        for (i, signal) in signals.iter().enumerate() {
            println!("  {}. UUID: {}", i + 1, signal.uuid);
            println!("     Symbol: {} ({:?})", signal.symbol_id, signal.symbol_kind);
            println!("     Hash (S): {}", signal.structural_hash);
            println!("     Hash (N): {}", signal.normalized_hash);
        }
        println!("📋 Verification complete. Status: OK");
    }
}
