use std::time::{Duration, SystemTime};
use vantage_core::EpochId;

/// Inference Avoidance Ratio:
/// IAR = 1 − (Inference_Vantage / Inference_Baseline)
///
/// Measures how much inference is eliminated by converting reasoning to state.
/// IAR → 1.0 means nearly all inference is replaced by state reads.
/// IAR → 0.0 means Vantage provides no inference savings.
pub fn compute_iar(inference_vantage: u64, inference_baseline: u64) -> f64 {
    if inference_baseline == 0 {
        return 0.0;
    }
    let ratio = inference_vantage as f64 / inference_baseline as f64;
    (1.0 - ratio).clamp(0.0, 1.0)
}

/// Reality Reuse Factor:
/// RRF = Reality Reads / Reality Constructions
///
/// Measures how many times a committed reality is read per construction.
/// RRF = 1 → each reality is used exactly once (no reuse).
/// RRF > 10 → each reality construction saves 10+ inference passes.
pub fn compute_rrf(reality_reads: u64, reality_constructions: u64) -> f64 {
    if reality_constructions == 0 {
        return 1.0;
    }
    reality_reads as f64 / reality_constructions as f64
}

/// Reality Yield:
/// RY = Tokens Saved / Reality Size (KB)
///
/// Economic efficiency of storing a reality.
/// High RY means a small stored reality replaces many tokens of inference.
pub fn compute_reality_yield(tokens_saved: u64, reality_bytes: u64) -> f64 {
    if reality_bytes == 0 {
        return 0.0;
    }
    let kb = reality_bytes as f64 / 1024.0;
    tokens_saved as f64 / kb.max(1.0)
}

/// Per-reality half-life tracking.
#[derive(Debug, Clone)]
pub struct HalfLifeObservation {
    pub reality_id: String,
    pub committed_at: SystemTime,
    pub invalidated_at: Option<SystemTime>,
    pub access_count: u64,
}

/// Compute the median half-life across a set of observations.
/// Half-life is defined as the time until 50% of realities are invalidated.
pub fn compute_half_life(observations: &[HalfLifeObservation]) -> Duration {
    if observations.is_empty() {
        return Duration::from_secs(0);
    }

    let mut lifetimes: Vec<Duration> = observations
        .iter()
        .filter_map(|o| {
            let end = o.invalidated_at.unwrap_or(o.committed_at);
            end.duration_since(o.committed_at).ok()
        })
        .collect();

    if lifetimes.is_empty() {
        return Duration::from_secs(0);
    }

    lifetimes.sort();
    lifetimes[lifetimes.len() / 2]
}

/// Aggregate metrics snapshot at a given epoch.
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub epoch: EpochId,
    pub inference_count_vantage: u64,
    pub inference_count_baseline: u64,
    pub state_reads: u64,
    pub reality_commits: u64,
    pub total_tokens_vantage: u64,
    pub total_tokens_baseline: u64,
    pub reality_bytes_stored: u64,
}

impl MetricsSnapshot {
    pub fn iar(&self) -> f64 {
        compute_iar(self.inference_count_vantage, self.inference_count_baseline)
    }

    pub fn rrf(&self) -> f64 {
        compute_rrf(self.state_reads, self.reality_commits)
    }

    pub fn reality_yield(&self) -> f64 {
        let tokens_saved = self
            .total_tokens_baseline
            .saturating_sub(self.total_tokens_vantage);
        compute_reality_yield(tokens_saved, self.reality_bytes_stored)
    }
}

/// Cumulative metrics across epochs.
#[derive(Debug, Clone)]
pub struct CumulativeMetrics {
    snapshots: Vec<MetricsSnapshot>,
}

impl CumulativeMetrics {
    pub fn new() -> Self {
        Self {
            snapshots: Vec::new(),
        }
    }

    pub fn record(&mut self, snapshot: MetricsSnapshot) {
        self.snapshots.push(snapshot);
    }

    pub fn latest(&self) -> Option<&MetricsSnapshot> {
        self.snapshots.last()
    }

    pub fn epoch_count(&self) -> usize {
        self.snapshots.len()
    }

    pub fn cumulative_iar(&self) -> f64 {
        let total_vantage: u64 = self
            .snapshots
            .iter()
            .map(|s| s.inference_count_vantage)
            .sum();
        let total_baseline: u64 = self
            .snapshots
            .iter()
            .map(|s| s.inference_count_baseline)
            .sum();
        compute_iar(total_vantage, total_baseline)
    }

    pub fn cumulative_rrf(&self) -> f64 {
        let total_reads: u64 = self.snapshots.iter().map(|s| s.state_reads).sum();
        let total_commits: u64 = self.snapshots.iter().map(|s| s.reality_commits).sum();
        compute_rrf(total_reads, total_commits)
    }

    pub fn iar_by_epoch(&self) -> Vec<(EpochId, f64)> {
        self.snapshots
            .iter()
            .map(|s| (s.epoch, s.iar()))
            .collect()
    }

    pub fn rrf_by_epoch(&self) -> Vec<(EpochId, f64)> {
        self.snapshots
            .iter()
            .map(|s| (s.epoch, s.rrf()))
            .collect()
    }

    pub fn total_tokens_saved(&self) -> u64 {
        let baseline: u64 = self.snapshots.iter().map(|s| s.total_tokens_baseline).sum();
        let vantage: u64 = self.snapshots.iter().map(|s| s.total_tokens_vantage).sum();
        baseline.saturating_sub(vantage)
    }

    pub fn cumulative_reality_yield(&self) -> f64 {
        let total_saved: u64 = self
            .snapshots
            .iter()
            .map(|s| {
                s.total_tokens_baseline
                    .saturating_sub(s.total_tokens_vantage)
            })
            .sum();
        let total_bytes: u64 = self.snapshots.iter().map(|s| s.reality_bytes_stored).sum();
        compute_reality_yield(total_saved, total_bytes)
    }
}

impl Default for CumulativeMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Per-node metric accumulator.
#[derive(Debug, Clone)]
pub struct NodeMetrics {
    pub node_id: String,
    pub cumulative: CumulativeMetrics,
    pub half_life_observations: Vec<HalfLifeObservation>,
}

impl NodeMetrics {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            cumulative: CumulativeMetrics::new(),
            half_life_observations: Vec::new(),
        }
    }

    pub fn record_half_life(&mut self, obs: HalfLifeObservation) {
        self.half_life_observations.push(obs);
    }

    pub fn half_life(&self) -> Duration {
        compute_half_life(&self.half_life_observations)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iar_perfect_savings() {
        let iar = compute_iar(0, 100);
        assert!((iar - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_iar_no_savings() {
        let iar = compute_iar(100, 100);
        assert!((iar - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_iar_partial() {
        let iar = compute_iar(30, 100);
        assert!((iar - 0.7).abs() < 1e-9);
    }

    #[test]
    fn test_iar_zero_baseline() {
        let iar = compute_iar(10, 0);
        assert!((iar - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_rrf_high_reuse() {
        let rrf = compute_rrf(100, 5);
        assert!((rrf - 20.0).abs() < 1e-9);
    }

    #[test]
    fn test_rrf_no_construction() {
        let rrf = compute_rrf(0, 0);
        assert!((rrf - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_reality_yield() {
        // 10000 tokens saved, 1KB stored
        let ry = compute_reality_yield(10000, 1024);
        assert!((ry - 10000.0).abs() < 1e-6);
    }

    #[test]
    fn test_cumulative_metrics() {
        let mut cum = CumulativeMetrics::new();
        cum.record(MetricsSnapshot {
            epoch: EpochId(1),
            inference_count_vantage: 10,
            inference_count_baseline: 50,
            state_reads: 40,
            reality_commits: 2,
            total_tokens_vantage: 1000,
            total_tokens_baseline: 5000,
            reality_bytes_stored: 512,
        });
        cum.record(MetricsSnapshot {
            epoch: EpochId(2),
            inference_count_vantage: 5,
            inference_count_baseline: 50,
            state_reads: 60,
            reality_commits: 1,
            total_tokens_vantage: 500,
            total_tokens_baseline: 5000,
            reality_bytes_stored: 256,
        });

        let iar = cum.cumulative_iar();
        assert!((iar - (1.0 - 15.0 / 100.0)).abs() < 1e-9);
        // (10+5)/(50+50) = 0.15 → IAR = 0.85

        let rrf = cum.cumulative_rrf();
        assert!((rrf - (100.0 / 3.0)).abs() < 1e-9);
        // (40+60)/(2+1) = 100/3 ≈ 33.3

        assert_eq!(cum.total_tokens_saved(), 8500);
    }

    #[test]
    fn test_half_life_median() {
        let now = SystemTime::now();
        let obs = vec![
            HalfLifeObservation {
                reality_id: "r1".into(),
                committed_at: now,
                invalidated_at: Some(now + Duration::from_secs(100)),
                access_count: 5,
            },
            HalfLifeObservation {
                reality_id: "r2".into(),
                committed_at: now,
                invalidated_at: Some(now + Duration::from_secs(200)),
                access_count: 3,
            },
            HalfLifeObservation {
                reality_id: "r3".into(),
                committed_at: now,
                invalidated_at: Some(now + Duration::from_secs(300)),
                access_count: 1,
            },
        ];
        let hl = compute_half_life(&obs);
        assert_eq!(hl, Duration::from_secs(200));
    }
}
