use crate::cognition::CognitiveSignal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftStatus {
    Unchanged,
    StructuralChange,
    SemanticChange,
    Added,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftItem {
    pub symbol_id: String,
    pub status: DriftStatus,
    pub old_hash: Option<String>,
    pub new_hash: Option<String>,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub total_symbols: usize,
    pub unchanged: usize,
    pub structural_changes: usize,
    pub semantic_changes: usize,
    pub added: usize,
    pub removed: usize,
    pub items: Vec<DriftItem>,
}

impl DriftReport {
    /// Compare current signals against baseline signals.
    /// Uses normalized_hash for rename-invariant comparison.
    pub fn compare(baseline: &[CognitiveSignal], current: &[CognitiveSignal]) -> Self {
        let mut items = Vec::new();
        let mut unchanged = 0;
        let mut structural_changes = 0;
        let mut semantic_changes = 0;
        let mut added = 0;
        let mut removed = 0;

        // Index baseline by symbol_id
        let baseline_map: std::collections::HashMap<&str, &CognitiveSignal> =
            baseline.iter().map(|s| (s.symbol_id.as_str(), s)).collect();

        // Index current by symbol_id
        let current_map: std::collections::HashMap<&str, &CognitiveSignal> =
            current.iter().map(|s| (s.symbol_id.as_str(), s)).collect();

        // Check current signals against baseline
        for sig in current {
            if let Some(baseline_sig) = baseline_map.get(sig.symbol_id.as_str()) {
                // Symbol exists in both - compare hashes
                let struct_changed = sig.structural_hash != baseline_sig.structural_hash;
                let norm_changed = sig.normalized_hash != baseline_sig.normalized_hash;

                let status = if !struct_changed && !norm_changed {
                    unchanged += 1;
                    DriftStatus::Unchanged
                } else if norm_changed {
                    semantic_changes += 1;
                    DriftStatus::SemanticChange
                } else {
                    structural_changes += 1;
                    DriftStatus::StructuralChange
                };

                items.push(DriftItem {
                    symbol_id: sig.symbol_id.clone(),
                    status,
                    old_hash: Some(baseline_sig.normalized_hash.clone()),
                    new_hash: Some(sig.normalized_hash.clone()),
                    location: format!(
                        "{}:{}:{}",
                        sig.location.file, sig.location.start_line, sig.location.start_col
                    ),
                });
            } else {
                // New symbol not in baseline
                added += 1;
                items.push(DriftItem {
                    symbol_id: sig.symbol_id.clone(),
                    status: DriftStatus::Added,
                    old_hash: None,
                    new_hash: Some(sig.normalized_hash.clone()),
                    location: format!(
                        "{}:{}:{}",
                        sig.location.file, sig.location.start_line, sig.location.start_col
                    ),
                });
            }
        }

        // Check for removed symbols
        for sig in baseline {
            if !current_map.contains_key(sig.symbol_id.as_str()) {
                removed += 1;
                items.push(DriftItem {
                    symbol_id: sig.symbol_id.clone(),
                    status: DriftStatus::Removed,
                    old_hash: Some(sig.normalized_hash.clone()),
                    new_hash: None,
                    location: format!(
                        "{}:{}:{}",
                        sig.location.file, sig.location.start_line, sig.location.start_col
                    ),
                });
            }
        }

        let total_symbols = items.len();
        DriftReport {
            total_symbols,
            unchanged,
            structural_changes,
            semantic_changes,
            added,
            removed,
            items,
        }
    }
}
