use crate::memory::MemoryPool;

// vantage:
//   invariant: DeterministicOrdering
//   reason: Task execution order must be reproducible
pub struct Scheduler {
    pool: MemoryPool,
    tasks: Vec<String>,
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            pool: MemoryPool::new(),
            tasks: Vec::new(),
        }
    }

    pub fn spawn(&mut self, name: &str) {
        self.pool.allocate(name.len());
        self.tasks.push(name.to_string());
    }

    pub fn run_all(&self) {
        for task in &self.tasks {
            println!("Running: {}", task);
        }
    }
}
