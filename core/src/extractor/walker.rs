use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Returns a list of target files sorted by path for determinism.
/// Automatically ignores: .git, venv, node_modules, target, __pycache__, .venv
pub fn find_target_files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    let mut builder = WalkBuilder::new(root);
    builder
        .sort_by_file_path(std::cmp::Ord::cmp)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .hidden(true)
        .filter_entry(|entry| {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !matches!(name, ".git" | "venv" | ".venv" | "node_modules" | "target" | "__pycache__" | ".pytest_cache" | ".mypy_cache" | "dist" | "build")
        });
    
    let walker = builder.build();

    for result in walker.flatten() {
        let path = result.path();
        if path.is_file()
            && path.extension().is_some_and(|ext| ext == extension)
        {
            files.push(path.to_path_buf());
        }
    }
    
    files
}