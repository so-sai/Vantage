use crate::arena::Arena;

// vantage:
//   invariant: ThreadSafe
//   reason: Shared memory pool accessed by concurrent schedulers
pub struct MemoryPool {
    arena: Arena<u8>,
    blocks: Vec<(usize, usize)>,
}

impl MemoryPool {
    pub fn new() -> Self {
        MemoryPool {
            arena: Arena::new(),
            blocks: Vec::new(),
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<usize> {
        let start = self.arena.alloc(0);
        for i in 0..size {
            self.arena.alloc(0);
        }
        let block_id = self.blocks.len();
        self.blocks.push((start, size));
        Some(block_id)
    }
}
