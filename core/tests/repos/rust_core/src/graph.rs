use crate::scheduler::Scheduler;

pub struct Node {
    pub id: u64,
    pub name: String,
}

pub struct Edge {
    pub from: u64,
    pub to: u64,
}

pub struct Graph {
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    scheduler: Scheduler,
}

impl Graph {
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            edges: Vec::new(),
            scheduler: Scheduler::new(),
        }
    }

    pub fn add_node(&mut self, name: &str) -> u64 {
        let id = self.nodes.len() as u64;
        self.nodes.push(Node { id, name: name.to_string() });
        self.scheduler.spawn(name);
        id
    }
}
