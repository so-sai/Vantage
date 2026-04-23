use std::path::Path;
use std::fs;
use anyhow::Result;
use std::io;

use super::{walker, emitter};

/// Main orchestration pipeline for edge extraction.
/// Follows O(1) memory streaming and deterministic source ordering.
pub fn run(target_dir: &Path) -> Result<()> {
    let mut stdout = io::stdout().lock();
    
    // 1. Find all target files
    let mut files = walker::find_target_files(target_dir, "py");
    files.extend(walker::find_target_files(target_dir, "rs"));
    files.extend(walker::find_target_files(target_dir, "rb"));
    files.extend(walker::find_target_files(target_dir, "js"));
    files.extend(walker::find_target_files(target_dir, "jsx"));
    files.extend(walker::find_target_files(target_dir, "ts"));
    files.extend(walker::find_target_files(target_dir, "tsx"));
    
    // 2. Sort files for determinism
    files.sort();
    
    for file in files {
        if let Ok(source_code) = fs::read_to_string(&file) {
            let file_path_str = file.to_string_lossy();
            let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("");
            
            let lang_name = match ext {
                "rs" => "rust",
                "py" => "python",
                "rb" => "ruby",
                "js" | "jsx" => "javascript",
                "ts" => "typescript",
                "tsx" => "tsx",
                _ => continue,
            };

            // 3. Use EpistemicParser for unified extraction (Phase C)
            let lang = crate::parser::Language::from_extension(ext).unwrap();
            let mut parser = crate::parser::get_parser(lang).map_err(anyhow::Error::msg)?;
            let (_, graph) = parser.parse_with_graph(&source_code, &file_path_str);

            // 4. Normalize Path (Relative + Unix style)
            let rel_path = if target_dir.is_file() {
                file.file_name().map(Path::new).unwrap_or(&file)
            } else {
                file.strip_prefix(target_dir).unwrap_or(&file)
            };
            let normalized_path = rel_path.to_string_lossy().replace("\\", "/");

            // 5. Emit edges as EdgeEvents (Sorted for Determinism)
            let mut sorted_edges = graph.unresolved_edges;
            sorted_edges.sort_by(|a, b| {
                let a_key = format!("{}-{}-{}", normalized_path, a.0.to_string(), a.1.to_string());
                let b_key = format!("{}-{}-{}", normalized_path, b.0.to_string(), b.1.to_string());
                a_key.cmp(&b_key)
            });

            for (from, to, kind) in sorted_edges {
                let event = vantage_types::EdgeEvent {
                    source: from.to_string(),
                    target: to.to_string(),
                    edge_type: match kind {
                        vantage_types::graph::DependencyKind::CallEdge => vantage_types::EdgeType::CallsUnresolved,
                        vantage_types::graph::DependencyKind::ModuleImport => vantage_types::EdgeType::Imports,
                        _ => vantage_types::EdgeType::CallsUnresolved,
                    },
                    language: lang_name.to_string(),
                    source_file: normalized_path.clone(),
                    target_file: None,
                    line: 0, 
                    confidence: 1.0,
                    raw_text: to.to_string(),
                };
                emitter::emit_edge(&event, &mut stdout)?;
            }
        }
    }
    
    Ok(())
}
