use crate::arena::NodeArena;
use crate::node_id::NodeId;
use crate::telemetry::PerfMetrics;

pub struct DirtyPropagator<'a> {
    arena: &'a mut NodeArena,
}

impl<'a> DirtyPropagator<'a> {
    pub fn new(arena: &'a mut NodeArena) -> Self {
        Self { arena }
    }

    /// Propagates dirty status from leaf nodes up to the root.
    /// Short-circuits as soon as a dirty node is encountered.
    /// Complexity: O(depth)
    pub fn mark_dirty(&mut self, leaf_id: NodeId, metrics: &mut PerfMetrics) -> usize {
        const MAX_DEPTH: usize = 4096;
        let mut current = leaf_id;
        let mut depth: usize = 0;

        loop {
            if depth >= MAX_DEPTH {
                // Safety break for cycles
                break;
            }

            let entry = match self.arena.get_mut(&current) {
                Some(e) => e,
                None => break,
            };

            // SHORT-CIRCUIT: Stop if we hit an already dirty node
            if entry.stamp.dirty {
                break;
            }

            entry.stamp.mark_dirty();
            depth += 1;

            // Track telemetry
            metrics.update_dirty_chain(depth as u16);

            match entry.node.parent_id {
                Some(parent) => {
                    current = parent;
                }
                None => {
                    break;
                }
            }
        }
        depth
    }
}
