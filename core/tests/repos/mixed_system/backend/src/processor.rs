use crate::arena::Arena;

pub fn process(items: Vec<String>) -> Arena<String> {
    let mut arena = Arena::new();
    for item in items {
        arena.push(item);
    }
    arena
}
