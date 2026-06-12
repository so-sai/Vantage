use std::collections::HashMap;
use vantage_core::{EpochId, ResourceId};
use vantage_metrics::{compute_iar, compute_rrf, MetricsSnapshot};

/// A single operation in a benchmark scenario.
#[derive(Debug, Clone)]
pub enum BenchmarkOp {
    /// Read a resource (repo, file, etc.)
    Read {
        resource: ResourceId,
        size_tokens: u64,
    },
    /// Search for a symbol or definition.
    Search {
        query: String,
        size_tokens: u64,
    },
    /// Edit a file.
    Edit {
        target: ResourceId,
        size_tokens: u64,
    },
    /// Re-query information that was previously discovered.
    RepeatQuery {
        query: String,
        resource: ResourceId,
        size_tokens: u64,
    },
    /// Commit a reality (Vantage-only).
    CommitReality {
        resource: ResourceId,
        size_bytes: u64,
    },
    /// Read from committed reality state (Vantage-only).
    ReadState {
        resource: ResourceId,
    },
}

/// Strategy for executing a benchmark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Every operation triggers full inference (Claude-style).
    InferenceOnly,
    /// Inference happens once; subsequent reads use committed state (Vantage).
    StateCached,
}

/// Result of a single benchmark run.
#[derive(Debug, Clone)]
pub struct BenchmarkRun {
    pub strategy: Strategy,
    pub total_inferences: u64,
    pub total_tokens: u64,
    pub state_reads: u64,
    pub reality_commits: u64,
    pub reality_bytes: u64,
}

impl BenchmarkRun {
    pub fn to_snapshot(&self, epoch: EpochId) -> MetricsSnapshot {
        MetricsSnapshot {
            epoch,
            inference_count_vantage: if self.strategy == Strategy::StateCached {
                self.total_inferences
            } else {
                0
            },
            inference_count_baseline: if self.strategy == Strategy::InferenceOnly {
                self.total_inferences
            } else {
                0
            },
            state_reads: self.state_reads,
            reality_commits: self.reality_commits,
            total_tokens_vantage: if self.strategy == Strategy::StateCached {
                self.total_tokens
            } else {
                0
            },
            total_tokens_baseline: if self.strategy == Strategy::InferenceOnly {
                self.total_tokens
            } else {
                0
            },
            reality_bytes_stored: self.reality_bytes,
        }
    }
}

/// Run a sequence of operations under a given strategy.
pub fn execute_benchmark(ops: &[BenchmarkOp], strategy: Strategy) -> BenchmarkRun {
    let mut total_inferences = 0u64;
    let mut total_tokens = 0u64;
    let mut state_reads = 0u64;
    let mut reality_commits = 0u64;
    let mut reality_bytes = 0u64;

    // Track what has been committed to state (Vantage-only).
    let mut committed: HashMap<ResourceId, bool> = HashMap::new();

    for op in ops {
        match op {
            BenchmarkOp::Read { size_tokens, .. }
            | BenchmarkOp::Search { size_tokens, .. }
            | BenchmarkOp::Edit { size_tokens, .. } => {
                // These always cost inference tokens in both strategies.
                total_inferences += 1;
                total_tokens += size_tokens;

                // Vantage commits reality after an edit.
                if let BenchmarkOp::Edit { target, .. } = op {
                    if strategy == Strategy::StateCached {
                        committed.insert(target.clone(), true);
                        reality_commits += 1;
                        // Estimate reality size from operation tokens.
                        let bytes = size_tokens * 10; // rough: 1 token ≈ 10 bytes stored
                        reality_bytes += bytes;
                    }
                }
            }

            BenchmarkOp::RepeatQuery { resource, size_tokens, .. } => {
                match strategy {
                    Strategy::InferenceOnly => {
                        // Must re-infer every time.
                        total_inferences += 1;
                        total_tokens += size_tokens;
                    }
                    Strategy::StateCached => {
                        if committed.contains_key(resource) {
                            // Already committed: just read state, no inference.
                            state_reads += 1;
                        } else {
                            // Not yet committed: must infer.
                            total_inferences += 1;
                            total_tokens += size_tokens;
                        }
                    }
                }
            }

            BenchmarkOp::CommitReality { size_bytes, .. } => {
                if strategy == Strategy::StateCached {
                    reality_commits += 1;
                    reality_bytes += size_bytes;
                }
            }

            BenchmarkOp::ReadState { resource } => {
                if strategy == Strategy::StateCached && committed.contains_key(resource) {
                    state_reads += 1;
                } else {
                    // Fall back to inference if state not available.
                    total_inferences += 1;
                    total_tokens += 100; // default cost
                }
            }
        }
    }

    BenchmarkRun {
        strategy,
        total_inferences,
        total_tokens,
        state_reads,
        reality_commits,
        reality_bytes,
    }
}

/// Compare two runs and produce IAR + delta metrics.
pub fn compare_runs(inference_only: &BenchmarkRun, vantage: &BenchmarkRun) -> ComparisonResult {
    assert_eq!(inference_only.strategy, Strategy::InferenceOnly);
    assert_eq!(vantage.strategy, Strategy::StateCached);

    ComparisonResult {
        iar: compute_iar(vantage.total_inferences, inference_only.total_inferences),
        rrf: compute_rrf(vantage.state_reads, vantage.reality_commits),
        tokens_saved: inference_only
            .total_tokens
            .saturating_sub(vantage.total_tokens),
        inference_saved: inference_only
            .total_inferences
            .saturating_sub(vantage.total_inferences),
        vantage_state_reads: vantage.state_reads,
        vantage_reality_commits: vantage.reality_commits,
    }
}

#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub iar: f64,
    pub rrf: f64,
    pub tokens_saved: u64,
    pub inference_saved: u64,
    pub vantage_state_reads: u64,
    pub vantage_reality_commits: u64,
}

/// Run a multi-epoch benchmark where operations repeat across epochs.
/// This simulates a long-lived agent that accumulates state over time.
pub fn run_multi_epoch_benchmark(
    per_epoch_ops: Vec<Vec<BenchmarkOp>>,
) -> Vec<(EpochId, BenchmarkRun, BenchmarkRun)> {
    let mut results = Vec::new();
    for (i, ops) in per_epoch_ops.into_iter().enumerate() {
        let epoch = EpochId(i as u64 + 1);
        let inference_run = execute_benchmark(&ops, Strategy::InferenceOnly);
        let vantage_run = execute_benchmark(&ops, Strategy::StateCached);
        results.push((epoch, inference_run, vantage_run));
    }
    results
}

/// Build a standard "agent edits a repo" benchmark sequence.
/// Simulates: read repo → find symbol → edit → re-query info → re-query again.
pub fn repo_edit_scenario(repeat_count: usize) -> Vec<BenchmarkOp> {
    let mut ops = Vec::new();

    // Initial read of repository.
    ops.push(BenchmarkOp::Read {
        resource: ResourceId("repo".into()),
        size_tokens: 50_000,
    });

    // Find a symbol.
    ops.push(BenchmarkOp::Search {
        query: "find fn authenticate in auth.rs".into(),
        size_tokens: 10_000,
    });

    // Edit the file.
    ops.push(BenchmarkOp::Edit {
        target: ResourceId("auth.rs".into()),
        size_tokens: 20_000,
    });

    // Repeated queries about the same logic.
    for i in 0..repeat_count {
        ops.push(BenchmarkOp::RepeatQuery {
            query: format!("what does authenticate do? (query #{})", i + 1),
            resource: ResourceId("auth.rs".into()),
            size_tokens: 5_000,
        });
    }

    ops
}

/// Scenario: multiple independent edits across different files.
/// Tests RRF when different realities are touched.
pub fn multi_file_edit_scenario(files: &[&str]) -> Vec<Vec<BenchmarkOp>> {
    files
        .iter()
        .map(|file| {
            let mut ops = Vec::new();
            ops.push(BenchmarkOp::Read {
                resource: ResourceId(format!("repo/{}", file)),
                size_tokens: 10_000,
            });
            ops.push(BenchmarkOp::Search {
                query: format!("find main struct in {}", file),
                size_tokens: 5_000,
            });
            ops.push(BenchmarkOp::Edit {
                target: ResourceId(file.to_string()),
                size_tokens: 15_000,
            });
            // Query twice about the edited file.
            for i in 0..2 {
                ops.push(BenchmarkOp::RepeatQuery {
                    query: format!("what changed in {}? (query #{})", file, i + 1),
                    resource: ResourceId(file.to_string()),
                    size_tokens: 3_000,
                });
            }
            ops
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_only_all_ops_cost_inference() {
        let ops = vec![
            BenchmarkOp::Read {
                resource: ResourceId("repo".into()),
                size_tokens: 100,
            },
            BenchmarkOp::Search {
                query: "search".into(),
                size_tokens: 50,
            },
            BenchmarkOp::Edit {
                target: ResourceId("file.rs".into()),
                size_tokens: 200,
            },
        ];
        let run = execute_benchmark(&ops, Strategy::InferenceOnly);
        assert_eq!(run.total_inferences, 3);
        assert_eq!(run.state_reads, 0);
        assert_eq!(run.reality_commits, 0);
    }

    #[test]
    fn test_state_cached_avoids_repeat_inference() {
        let ops = vec![
            BenchmarkOp::Read {
                resource: ResourceId("repo".into()),
                size_tokens: 100,
            },
            BenchmarkOp::Edit {
                target: ResourceId("file.rs".into()),
                size_tokens: 200,
            },
            BenchmarkOp::RepeatQuery {
                query: "q1".into(),
                resource: ResourceId("file.rs".into()),
                size_tokens: 50,
            },
            BenchmarkOp::RepeatQuery {
                query: "q2".into(),
                resource: ResourceId("file.rs".into()),
                size_tokens: 50,
            },
        ];
        let vantage = execute_benchmark(&ops, Strategy::StateCached);
        // 2 inferences (read + edit), 2 state reads (repeat queries)
        assert_eq!(vantage.total_inferences, 2);
        assert_eq!(vantage.state_reads, 2);
        assert_eq!(vantage.reality_commits, 1);

        let baseline = execute_benchmark(&ops, Strategy::InferenceOnly);
        assert_eq!(baseline.total_inferences, 4);
        assert_eq!(baseline.state_reads, 0);
    }

    #[test]
    fn test_state_cached_miss_falls_back_to_inference() {
        let ops = vec![
            BenchmarkOp::ReadState {
                resource: ResourceId("unknown.rs".into()),
            },
        ];
        let vantage = execute_benchmark(&ops, Strategy::StateCached);
        // State not committed → falls back to inference.
        assert_eq!(vantage.total_inferences, 1);
        assert_eq!(vantage.state_reads, 0);
    }

    #[test]
    fn test_compare_shows_savings() {
        let ops = repo_edit_scenario(5);
        let baseline = execute_benchmark(&ops, Strategy::InferenceOnly);
        let vantage = execute_benchmark(&ops, Strategy::StateCached);
        let cmp = compare_runs(&baseline, &vantage);

        // 3 initial ops + 5 repeats = 8 inferences baseline
        // 3 initial + 0 repeats with state = 3 inferences vantage
        assert_eq!(baseline.total_inferences, 8);
        assert_eq!(vantage.total_inferences, 3);
        assert_eq!(cmp.inference_saved, 5);
        assert!(cmp.iar > 0.6);
    }

    #[test]
    fn test_multi_epoch_benchmark_rrf_increases() {
        let scenario = multi_file_edit_scenario(&["auth.rs", "db.rs", "api.rs"]);
        let results = run_multi_epoch_benchmark(scenario);

        // Each epoch has: read + search + edit + 2 repeats = 5 ops
        // Baseline: 5 inferences per epoch
        // Vantage: 3 inferences (read+search+edit) + 2 state reads = 3 inferences
        for (_, baseline, vantage) in &results {
            assert_eq!(baseline.total_inferences, 5);
            assert_eq!(vantage.total_inferences, 3);
            assert_eq!(vantage.state_reads, 2);
        }

        // RRF = 2 state reads / 1 commit = 2.0 per epoch
        let cmp = compare_runs(&results[0].1, &results[0].2);
        assert!((cmp.rrf - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_repo_edit_iar_improves_with_more_repeats() {
        // 1 repeat: low savings
        let ops_1 = repo_edit_scenario(1);
        let b1 = execute_benchmark(&ops_1, Strategy::InferenceOnly);
        let v1 = execute_benchmark(&ops_1, Strategy::StateCached);
        let cmp1 = compare_runs(&b1, &v1);
        assert!((cmp1.iar - 0.25).abs() < 1e-9); // 3/4 = 0.75 → IAR = 0.25

        // 10 repeats: high savings
        let ops_10 = repo_edit_scenario(10);
        let b10 = execute_benchmark(&ops_10, Strategy::InferenceOnly);
        let v10 = execute_benchmark(&ops_10, Strategy::StateCached);
        let cmp10 = compare_runs(&b10, &v10);
        // 3 inferences / 13 inferences = 0.23 → IAR = 0.77
        assert!(cmp10.iar > 0.7);
        assert!(cmp10.iar > cmp1.iar); // IAR improves with more repeats
    }
}
