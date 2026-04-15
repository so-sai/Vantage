use crate::cognition::CognitiveSignal;
use serde::{Deserialize, Serialize};
use vantage_types::SymbolId;

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
    pub symbol_id: SymbolId,
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
    pub added: usize,
    pub removed: usize,
    pub items: Vec<DriftItem>,
}

impl DriftReport {
    /// Compare current signals against baseline signals.
    /// Uses SymbolId as primary key (stable identity) to handle hash logic changes.
    pub fn compare(baseline: &[CognitiveSignal], current: &[CognitiveSignal]) -> Self {
        let mut items = Vec::new();
        let mut unchanged = 0;
        let mut structural_changes = 0;
        let mut added = 0;
        let mut removed = 0;

        // Index baseline by SymbolId (stable identity)
        let baseline_map: std::collections::HashMap<&SymbolId, &CognitiveSignal> =
            baseline.iter().map(|s| (&s.symbol_id, s)).collect();

        // Index current by SymbolId
        let current_map: std::collections::HashMap<&SymbolId, &CognitiveSignal> =
            current.iter().map(|s| (&s.symbol_id, s)).collect();

        // Check current signals against baseline by symbol identity
        for sig in current {
            if let Some(baseline_sig) = baseline_map.get(&sig.symbol_id) {
                // Symbol exists in both - compare structural_hash only (not normalized_hash)
                // Rationale: normalized_hash may change when hash logic changes
                // but structural_hash (physical content) should still reflect actual changes
                let struct_changed = sig.structural_hash != baseline_sig.structural_hash;

                let status = if struct_changed {
                    structural_changes += 1;
                    DriftStatus::StructuralChange
                } else {
                    unchanged += 1;
                    DriftStatus::Unchanged
                };

                items.push(DriftItem {
                    symbol_id: sig.symbol_id.clone(),
                    status,
                    old_hash: Some(baseline_sig.structural_hash.clone()),
                    new_hash: Some(sig.structural_hash.clone()),
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
            if !current_map.contains_key(&sig.symbol_id) {
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
            added,
            removed,
            items,
        }
    }
}
