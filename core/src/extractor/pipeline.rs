use std::path::Path;
use std::fs;
use anyhow::Result;
use std::io;

use super::{walker, python, emitter};

/// Main orchestration pipeline for edge extraction.
/// Follows O(1) memory streaming and deterministic source ordering.
pub fn run(target_dir: &Path) -> Result<()> {
    let mut stdout = io::stdout().lock();
    
    // 1. Get files sorted by path (Determinism)
    let files = walker::find_target_files(target_dir, "py");
    
    for file in files {
        // 2. Read file content
        if let Ok(source_code) = fs::read_to_string(&file) {
            let file_path_str = file.to_string_lossy();
            
            // 3. Extract imports (Deterministic top-down traversal)
            let edges = python::extract_imports(&source_code, &file_path_str);
            
            // 4. Emit JSONL (O(1) memory)
            for edge in edges {
                emitter::emit_edge(&edge, &mut stdout)?;
            }
        }
    }
    
    Ok(())
}
