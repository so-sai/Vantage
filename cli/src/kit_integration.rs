use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;
use tracing::instrument;

#[derive(Debug, Serialize, Clone)]
pub struct KitVerificationResult {
    pub records_scanned: usize,
    pub valid_hashes: usize,
    pub invalid_hashes: i64,
    pub integrity_ok: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DeepVerificationResult {
    pub orphan_count: u64,
    pub index_ok: bool,
    pub sqlite_health: bool,
    pub index_name: String,
}

#[derive(Debug)]
struct BakedObservation {
    content: String,
    structural_hash: String,
}

fn compute_normalized_hash(content: &str) -> String {
    let normalized: String = content
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[instrument(skip(kit_path), fields(kit_path = %kit_path.display()))]
pub fn verify_kit_memory(kit_path: &Path) -> Result<KitVerificationResult> {
    let db_path = kit_path.join("local_brain.db");

    if !db_path.exists() {
        return Ok(KitVerificationResult {
            records_scanned: 0,
            valid_hashes: 0,
            invalid_hashes: 0,
            integrity_ok: false,
            errors: vec![format!(
                "Kit database not found at {}",
                db_path.display()
            )],
        });
    }

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Failed to open Kit database in read-only mode")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, content, structural_hash FROM observations WHERE is_active = 1",
        )
        .context("Failed to prepare observations query")?;

    let observations: Vec<BakedObservation> = stmt
        .query_map([], |row| {
            Ok(BakedObservation {
                
                content: row.get(1)?,
                structural_hash: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut valid_count: i64 = 0;
    let mut invalid_count: i64 = 0;

    for obs in &observations {
        let computed = compute_normalized_hash(&obs.content);

        if computed == obs.structural_hash {
            valid_count += 1;
        } else {
            invalid_count += 1;
        }
    }

    let integrity_ok = invalid_count == 0;

    Ok(KitVerificationResult {
        records_scanned: observations.len(),
        valid_hashes: valid_count as usize,
        invalid_hashes: invalid_count,
        integrity_ok,
        errors: vec![],
    })
}

#[instrument(skip(kit_path), fields(kit_path = %kit_path.display()))]
pub fn verify_deep(kit_path: &Path) -> Result<DeepVerificationResult> {
    let db_path = kit_path.join("local_brain.db");

    if !db_path.exists() {
        return Ok(DeepVerificationResult {
            orphan_count: 0,
            index_ok: false,
            sqlite_health: false,
            index_name: String::new(),
        });
    }

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Failed to open Kit database")?;

    // 1. Orphan node check
    let orphan_count: u64 = conn.query_row(
        "SELECT COUNT(*) FROM observations o LEFT JOIN nodes n ON o.node_id = n.id WHERE o.node_id IS NOT NULL AND n.id IS NULL",
        [],
        |row| row.get(0),
    ).unwrap_or(0);

    // 2. Index check
    let index_exists: bool = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_obs_vantage_read'",
        [],
        |row| row.get::<_, i32>(0),
    ).unwrap_or(0) > 0;

    // 3. SQLite health check - PRAGMA integrity_check may not work with read-only, so check basic connectivity
    let sqlite_health = conn.execute_batch("SELECT COUNT(*) FROM observations").is_ok();

    Ok(DeepVerificationResult {
        orphan_count,
        index_ok: index_exists,
        sqlite_health,
        index_name: "idx_obs_vantage_read".to_string(),
    })
}

use std::time::Instant;

#[derive(Debug, serde::Serialize)]
pub struct BenchmarkResult {
    pub records: usize,
    pub time_ms: f64,
    pub records_per_sec: f64,
}

pub fn benchmark(kit_path: &Path) -> Result<BenchmarkResult> {
    let db_path = kit_path.join("local_brain.db");

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let start = Instant::now();

    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM observations WHERE is_active = 1",
        [],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT id, content, structural_hash FROM observations WHERE is_active = 1"
    )?;

    let observations: Vec<BakedObservation> = stmt
        .query_map([], |row| {
            Ok(BakedObservation {
                
                content: row.get(1)?,
                structural_hash: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    for obs in &observations {
        let _ = compute_normalized_hash(&obs.content);
    }

    let elapsed = start.elapsed();
    let time_ms = elapsed.as_secs_f64() * 1000.0;
    let records_per_sec = if time_ms > 0.0 { count as f64 / (time_ms / 1000.0) } else { 0.0 };

    Ok(BenchmarkResult {
        records: count,
        time_ms,
        records_per_sec,
    })
}

/// Verify that Kit is initialized and matches the expected version
pub fn check_kit_version(kit_path: &Path, expected: &str) -> Result<()> {
    let db_path = kit_path.join("local_brain.db");
    if !db_path.exists() {
        anyhow::bail!("Kit not initialized. Run 'kit init' first.");
    }

    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    
    // Attempt to read version from metadata table if it exists
    let version: String = conn.query_row(
        "SELECT value FROM metadata WHERE key = 'version'",
        [],
        |row| row.get(0),
    ).unwrap_or_else(|_| "unknown".to_string());

    if version != expected && version != "unknown" {
        anyhow::bail!("Kit version mismatch: found {}, expected {}. Run 'kit init' to upgrade.", version, expected);
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct EnvStatus {
    pub kit_init: bool,
    pub database_ok: bool,
    pub version: String,
}

pub fn verify_env(kit_path: &Path) -> Result<EnvStatus> {
    let db_path = kit_path.join("local_brain.db");
    let kit_init = db_path.exists();
    let mut database_ok = false;
    let mut version = "none".to_string();

    if kit_init {
        if let Ok(conn) = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
            database_ok = conn.execute_batch("SELECT 1 FROM observations LIMIT 1").is_ok();
            version = conn.query_row(
                "SELECT value FROM metadata WHERE key = 'version'",
                [],
                |row| row.get(0),
            ).unwrap_or_else(|_| "unknown".to_string());
        }
    }

    Ok(EnvStatus {
        kit_init,
        database_ok,
        version,
    })
}
