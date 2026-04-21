use std::fs;
use std::process::Command;
use sha2::{Sha256, Digest};
use tempfile::tempdir;

#[test]
fn test_byte_level_reproducibility() {
    // Create a temporary directory for the test
    let dir = tempdir().expect("Failed to create temp dir");
    let examples_dir = dir.path().join("examples");
    fs::create_dir(&examples_dir).expect("Failed to create examples dir");

    // Create a test file
    let code = r#"
import os
from datetime import datetime

class Base:
    pass

class Sub(Base):
    pass
"#;
    let file_path = examples_dir.join("test_repro.py");
    fs::write(&file_path, code).expect("Failed to write test file");

    // Function to run extraction and return hash of output
    let run_extraction = || {
        let output = Command::new("cargo")
            .args(["run", "--package", "vantage-cli", "--", "extract-edges"])
            .arg(examples_dir.to_str().unwrap())
            .output()
            .expect("Failed to run vantage-cli");
        
        assert!(output.status.success(), "vantage-cli failed: {}", String::from_utf8_lossy(&output.stderr));
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut hasher = Sha256::new();
        hasher.update(stdout.as_bytes());
        format!("{:x}", hasher.finalize())
    };

    // Run 5 times and ensure hash is identical
    let first_hash = run_extraction();
    for i in 1..5 {
        let next_hash = run_extraction();
        assert_eq!(first_hash, next_hash, "Determinism broken at run {}", i);
    }
}
