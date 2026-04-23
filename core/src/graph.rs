use std::collections::{HashMap, HashSet};
use vantage_types::symbol_id::SymbolId;
use vantage_types::graph::{SymbolState, DependencyKind, DependencyEdge};

/// The Authoritative DepNode (L1 Structural Layer)
/// Strictly forensic and engine-owned.
#[derive(Debug)]
pub struct DepNode {
    pub symbol: SymbolId,
    pub state: SymbolState,

    /// Symbols this node depends on (Outgoing edges)
    pub dependencies: HashSet<DependencyEdge>,

    /// Symbols that depend on this node (Incoming edges for reverse dirty prop)
    pub dependents: HashSet<SymbolId>,

    /// Metadata for visualization/tracking
    pub file: String,
    pub start_line: u32,

    /// Arena generation of the last discovery (Used for graceful tombstone eviction)
    pub generation: u32,
}

impl DepNode {
    pub fn new(symbol: SymbolId, file: &str, line: u32, generation: u32) -> Self {
        Self {
            symbol,
            state: SymbolState::Discovered,
            dependencies: HashSet::new(),
            dependents: HashSet::new(),
            file: file.to_string(),
            start_line: line,
            generation,
        }
    }
}

/// The Single Source of Truth for the Semantic Graph (v1.2.4)
/// Owned by vantage-core, provides the memory kernel for the Sensor Brain.
pub struct SymbolDependencyGraph {
    /// Maps symbol identity to its structural node
    pub nodes: HashMap<SymbolId, DepNode>,

    /// Raw edges that don't require node resolution (O(1) Sensor mode)
    pub unresolved_edges: Vec<(SymbolId, SymbolId, DependencyKind)>,

    /// Current analysis generation
    current_generation: u32,
}

impl SymbolDependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            unresolved_edges: Vec::new(),
            current_generation: 0,
        }
    }

    pub fn bump_generation(&mut self) {
        self.current_generation += 1;
    }

    /// Register a symbol discovery.
    pub fn add_node(&mut self, symbol: SymbolId, file: &str, line: u32) {
        let generation = self.current_generation;
        self.nodes.entry(symbol.clone())
            .and_modify(|node| {
                node.generation = generation;
                node.state = SymbolState::Discovered;
            })
            .or_insert_with(|| DepNode::new(symbol, file, line, generation));
    }

    /// Register a dependency relationship.
    /// Register a dependency relationship.
    pub fn add_edge(&mut self, from: &SymbolId, to: &SymbolId, kind: DependencyKind) {
        if from.identity_eq(to) {
            return;
        }

        // 1. Store in the flat list for O(1) Sensor mode (Unresolved emission)
        self.unresolved_edges.push((from.clone(), to.clone(), kind));

        // 2. Add outgoing edge to 'from' node (if it exists)
        if let Some(node) = self.nodes.get_mut(from) {
            node.dependencies.insert(DependencyEdge {
                target: to.clone(),
                kind,
            });
        }

        // 3. Add incoming edge to 'to' node (reverse tracking, if it exists)
        if let Some(node) = self.nodes.get_mut(to) {
            node.dependents.insert(from.clone());
        }
    }

    /// Impact radius (Upstream): who depends on this symbol?
    pub fn impact_radius(&self, symbol: &SymbolId) -> Vec<SymbolId> {
        self.nodes.get(symbol)
            .map(|node| node.dependents.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Downstream: what does this symbol depend on?
    pub fn downstream(&self, symbol: &SymbolId) -> Vec<SymbolId> {
        self.nodes.get(symbol)
            .map(|node| node.dependencies.iter().map(|e| e.target.clone()).collect())
            .unwrap_or_default()
    }

    /// Evicts Tombstoned nodes from the graph if they are older than the grace period (1 generation).
    pub fn gc(&mut self) {
        self.nodes.retain(|_, node| {
            if node.state == SymbolState::Tombstoned {
                // Must be at least 1 generation old before eviction
                node.generation >= self.current_generation
            } else {
                true
            }
        });
    }

    /// Marks nodes as Tombstoned if they were not seen in the current generation pass.
    pub fn mark_tombstones(&mut self) {
        for node in self.nodes.values_mut() {
            if node.generation < self.current_generation && node.state != SymbolState::Tombstoned {
                node.state = SymbolState::Tombstoned;
                // Update generation so GC gives it a lifecycle grace pass
                node.generation = self.current_generation;
            }
        }
    }

    /// Export to DTO for transport to Kit/Agent.
    /// Deterministic: nodes are sorted by SymbolId.
    pub fn to_dto(&self) -> vantage_types::graph::SymbolGraphDTO {
        use vantage_types::graph::{SymbolGraphDTO, SymbolNodeDTO};
        
        let mut nodes: Vec<SymbolNodeDTO> = self.nodes.values()
            .map(|n| SymbolNodeDTO {
                symbol: n.symbol.clone(),
                state: n.state,
                dependencies: {
                    let mut d = n.dependencies.iter().cloned().collect::<Vec<_>>();
                    d.sort_by(|a, b| a.target.to_string().cmp(&b.target.to_string()));
                    d
                },
                dependents: {
                    let mut d = n.dependents.iter().cloned().collect::<Vec<_>>();
                    d.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
                    d
                },
            })
            .collect();
        
        nodes.sort_by(|a, b| a.symbol.to_string().cmp(&b.symbol.to_string()));

        SymbolGraphDTO { nodes }
    }
}

impl Default for SymbolDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}
