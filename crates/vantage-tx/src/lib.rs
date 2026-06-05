use std::collections::{HashMap, HashSet};
use vantage_core::{
    KnowledgeMutation, MutationId, ResourceId, MutationOp, EpistemicReader, 
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    ReadAfterWrite,
    WriteAfterRead,
    WriteAfterWrite,
}

pub struct DependencyEdge {
    pub from: MutationId,
    pub to: MutationId,
    pub dep_type: DependencyType,
}

pub struct ResourceAccessSet {
    pub reads: HashSet<ResourceId>,
    pub writes: HashSet<ResourceId>,
}

impl ResourceAccessSet {
    pub fn from_mutation(m: &KnowledgeMutation) -> Self {
        let mut reads = HashSet::new();
        let mut writes = HashSet::new();
        match &m.op {
            MutationOp::Insert { resource_id, .. } => {
                writes.insert(resource_id.clone());
            }
            MutationOp::Delete { resource_id } => {
                writes.insert(resource_id.clone());
                reads.insert(resource_id.clone());
            }
        }
        Self { reads, writes }
    }
}

pub struct TransactionDAG {
    pub nodes: HashMap<MutationId, (KnowledgeMutation, ResourceAccessSet)>,
    pub adjacency: HashMap<MutationId, Vec<DependencyEdge>>,
}

impl TransactionDAG {
    pub fn compile(mutations: Vec<KnowledgeMutation>) -> Result<Self, String> {
        let mut nodes = HashMap::new();
        let mut adjacency = HashMap::new();
        let mut list = Vec::new();

        for m in mutations {
            let id = m.mutation_id.clone();
            let access_set = ResourceAccessSet::from_mutation(&m);
            nodes.insert(id.clone(), (m, access_set));
            adjacency.insert(id.clone(), Vec::new());
            list.push(id);
        }

        for i in 0..list.len() {
            for j in (i + 1)..list.len() {
                let id_i = &list[i];
                let id_j = &list[j];
                let (_, access_i) = &nodes[id_i];
                let (_, access_j) = &nodes[id_j];

                if !access_i.writes.intersection(&access_j.reads).cloned().collect::<HashSet<_>>().is_empty() {
                    adjacency.get_mut(id_i).unwrap().push(DependencyEdge {
                        from: id_i.clone(), to: id_j.clone(), dep_type: DependencyType::ReadAfterWrite
                    });
                    continue;
                }
                if !access_i.reads.intersection(&access_j.writes).cloned().collect::<HashSet<_>>().is_empty() {
                    adjacency.get_mut(id_i).unwrap().push(DependencyEdge {
                        from: id_i.clone(), to: id_j.clone(), dep_type: DependencyType::WriteAfterRead
                    });
                    continue;
                }
                if !access_i.writes.intersection(&access_j.writes).cloned().collect::<HashSet<_>>().is_empty() {
                    adjacency.get_mut(id_i).unwrap().push(DependencyEdge {
                        from: id_i.clone(), to: id_j.clone(), dep_type: DependencyType::WriteAfterWrite
                    });
                }
            }
        }

        let dag = Self { nodes, adjacency };
        dag.detect_cycles()?;
        Ok(dag)
    }

    pub fn topological_sort(&self) -> Vec<MutationId> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        for id in self.nodes.keys() {
            if !visited.contains(id) {
                self.dfs_sort(id, &mut visited, &mut order);
            }
        }
        order.reverse();
        order
    }

    fn dfs_sort(&self, node: &MutationId, visited: &mut HashSet<MutationId>, order: &mut Vec<MutationId>) {
        visited.insert(node.clone());
        if let Some(edges) = self.adjacency.get(node) {
            for edge in edges {
                if !visited.contains(&edge.to) {
                    self.dfs_sort(&edge.to, visited, order);
                }
            }
        }
        order.push(node.clone());
    }

    fn detect_cycles(&self) -> Result<(), String> {
        let mut colors = HashMap::new();
        for id in self.nodes.keys() { colors.insert(id.clone(), 0u8); }
        let mut path = Vec::new();
        for id in self.nodes.keys() {
            if colors[id] == 0u8 { self.dfs_cycle_check(id, &mut colors, &mut path)?; }
        }
        Ok(())
    }

    fn dfs_cycle_check(&self, node: &MutationId, colors: &mut HashMap<MutationId, u8>, path: &mut Vec<MutationId>) -> Result<(), String> {
        colors.insert(node.clone(), 1u8);
        path.push(node.clone());
        if let Some(edges) = self.adjacency.get(node) {
            for edge in edges {
                match colors[&edge.to] {
                    1 => return Err(format!("Cycle detected at: {:?}", edge.to)),
                    0 => { self.dfs_cycle_check(&edge.to, colors, path)?; }
                    _ => {}
                }
            }
        }
        path.pop();
        colors.insert(node.clone(), 2u8);
        Ok(())
    }
}

pub struct TransactionalView<'a, B: EpistemicReader> {
    pub base: &'a B,
    pub overlay: HashMap<ResourceId, Option<String>>,
}

impl<'a, B: EpistemicReader> TransactionalView<'a, B> {
    pub fn new(base: &'a B) -> Self {
        Self { base, overlay: HashMap::new() }
    }
    pub fn stage_write(&mut self, id: ResourceId, content: String) {
        self.overlay.insert(id, Some(content));
    }
    pub fn stage_delete(&mut self, id: ResourceId) {
        self.overlay.insert(id, None);
    }
}

impl<'a, B: EpistemicReader> EpistemicReader for TransactionalView<'a, B> {
    fn read_unit(&self, id: &ResourceId) -> Option<String> {
        match self.overlay.get(id) {
            Some(Some(content)) => Some(content.clone()),
            Some(None) => None,
            None => self.base.read_unit(id),
        }
    }
    fn exists(&self, id: &ResourceId) -> bool {
        match self.overlay.get(id) {
            Some(staged) => staged.is_some(),
            None => self.base.exists(id),
        }
    }
}
