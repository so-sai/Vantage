// vantage:
//   invariant: AppendOnly
//   reason: Core data structure for mixed-system demo
pub struct Arena<T> {
    items: Vec<T>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Arena { items: Vec::new() }
    }

    pub fn push(&mut self, item: T) {
        self.items.push(item);
    }
}
