use std::env;
use std::path::Path;
use vantage_core::parser::{Language, get_parser};

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut use_json = false;
    let mut use_debug = false;

    // Very basic arg parsing for v1.2.3
    let mut file_path = None;
    for arg in args.iter().skip(1) {
        if arg == "--json" {
            use_json = true;
        } else if arg == "--debug" {
            use_debug = true;
        } else if file_path.is_none() {
            file_path = Some(arg);
        }
    }

    let file_path = match file_path {
        Some(p) => p,
        None => {
            if use_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "reason": "Missing file path argument"
                    })
                );
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
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "reason": format!("Unsupported file extension: {}", ext)
                    })
                );
            } else {
                eprintln!("❌ Unsupported file extension: {}", ext);
            }
            std::process::exit(1);
        }
    };

    let mut parser = match get_parser(lang) {
        Ok(p) => p,
        Err(e) => {
            if use_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "reason": format!("Internal error: {}", e)
                    })
                );
            } else {
                eprintln!("❌ Internal error: {}", e);
            }
            std::process::exit(1);
        }
    };

    let content = match std::fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            if use_json {
                println!(
                    "{}",
                    serde_json::json!({
                        "status": "error",
                        "reason": format!("Error reading file: {}", e)
                    })
                );
            } else {
                eprintln!("❌ Error reading file: {}", e);
            }
            std::process::exit(1);
        }
    };

    let signals = parser.parse_signals(&content, file_path);

    if use_debug {
        println!("🔧 DEBUG: AST Tree for {}", file_path);
        let mut p = tree_sitter::Parser::new();
        let lang_ts = match lang {
            Language::Rust => tree_sitter_rust::LANGUAGE.into(),
            Language::Python => tree_sitter_python::LANGUAGE.into(),
        };
        let _ = p.set_language(&lang_ts);
        let tree = match p.parse(&content, None) {
            Some(t) => t,
            None => {
                eprintln!("❌ DEBUG: Parse failed");
                std::process::exit(1);
            }
        };
        print_tree(tree.root_node(), 0);
        println!("--- End of Tree ---");
    }

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
                    "semantic_hash": s.semantic_hash,
                    "normalized_hash": s.normalized_hash,
                    "signature": s.signature,
                    "location": {
                        "file": s.location.file,
                        "start_line": s.location.start_line,
                        "start_col": s.location.start_col,
                        "end_line": s.location.end_line,
                        "end_col": s.location.end_col,
                        "byte_start": s.location.byte_start,
                        "byte_end": s.location.byte_end,
                    }
                })
            }).collect::<Vec<_>>(),
            "reason": reason
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        );
    } else {
        println!("🛡️  VANTAGE STRUCTURAL VERIFY (v1.2.3-alpha)  🛡️");
        println!("📁 File: {}", file_path);

        if signals.is_empty() {
            println!("✅ No structural anchors found (@epistemic tag missing)");
            return;
        }

        println!("🔍 Found {} structural signal(s)", signals.len());
        for (i, signal) in signals.iter().enumerate() {
            println!("  {}. UUID: {}", i + 1, signal.uuid);
            println!(
                "     Symbol: {} ({:?})",
                signal.symbol_id, signal.symbol_kind
            );
            println!(
                "     Location: {}:{}:{}-{}:{}",
                signal.location.file,
                signal.location.start_line,
                signal.location.start_col,
                signal.location.end_line,
                signal.location.end_col
            );
            println!("     Hash (S): {}", &signal.structural_hash[..16]);
            println!("     Hash (N): {}", &signal.semantic_hash[..16]);
            println!("     Hash (A): {}", &signal.normalized_hash[..16]);
        }
        println!("📋 Verification complete. Status: OK");
    }
}

fn print_tree(node: tree_sitter::Node, depth: usize) {
    println!(
        "{}{} [{}-{}] \"{}\"",
        "  ".repeat(depth),
        node.kind(),
        node.start_byte(),
        node.end_byte(),
        node.kind()
    );
    for child in node.children(&mut node.walk()) {
        print_tree(child, depth + 1);
    }
}
