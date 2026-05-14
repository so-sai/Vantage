// vantage:
//   invariant: AppendOnly
//   reason: Prevent rollback corruption in memory arena
pub struct Arena<T> {
    items: Vec<T>,
    generation: u64,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Arena { items: Vec::new(), generation: 0 }
    }

    pub fn alloc(&mut self, item: T) -> usize {
        self.items.push(item);
        self.items.len() - 1
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.items.get(index)
    }
}
