use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerfMetrics {
    /// Total number of nodes recomputed in the current incremental pass
    pub nodes_recomputed: u32,

    /// Total number of nodes reused via cache
    pub nodes_reused: u32,

    /// Maximum depth reached during traversal
    pub max_depth: u16,

    /// Longest continuous dirty chain
    pub max_dirty_chain: u16,

    /// Total time spent in nanoseconds (monotonic)
    pub latency_ns: u64,

    /// Nodes skipped due to short-circuit
    pub nodes_skipped: u32,

    /// Memory delta during the pass (if measurable)
    pub memory_delta_bytes: Option<i64>,

    #[serde(skip)]
    start_time: Option<std::time::Instant>,
}


impl PerfMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_timer(&mut self) {
        self.start_time = Some(std::time::Instant::now());
    }

    pub fn stop_timer(&mut self) {
        if let Some(start) = self.start_time {
            self.latency_ns = start.elapsed().as_nanos() as u64;
        }
    }

    pub fn record_recompute(&mut self) {
        self.nodes_recomputed += 1;
    }

    pub fn record_reuse(&mut self) {
        self.nodes_reused += 1;
    }

    pub fn record_skip(&mut self) {
        self.nodes_skipped += 1;
    }

    pub fn update_depth(&mut self, depth: u16) {
        if depth > self.max_depth {
            self.max_depth = depth;
        }
    }

    pub fn update_dirty_chain(&mut self, chain: u16) {
        if chain > self.max_dirty_chain {
            self.max_dirty_chain = chain;
        }
    }

    pub fn reuse_ratio(&self) -> f64 {
        let total = self.nodes_reused + self.nodes_recomputed;
        if total == 0 {
            return 1.0;
        }
        self.nodes_reused as f64 / total as f64
    }

    pub fn bump(&mut self) {
        let start = self.start_time;
        *self = Self::default();
        self.start_time = start;
    }

    pub fn report(&self) {
        let ratio = self.reuse_ratio() * 100.0;
        if self.latency_ns > 5_000_000 {
            println!(
                "[VANTAGE-ALARM] Slow incremental pass: {} recomputed, {}ns, {:.2}% reuse",
                self.nodes_recomputed, self.latency_ns, ratio
            );
        }
    }
}
