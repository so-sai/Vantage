use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Returns a list of target files sorted by path for determinism.
pub fn find_target_files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    
    let mut builder = WalkBuilder::new(root);
    // Enforce determinism: sort by file path
    builder.sort_by_file_path(std::cmp::Ord::cmp);
    
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
