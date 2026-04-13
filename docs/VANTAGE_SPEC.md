# Vantage v1.2.4 — Structural Identity Engine Spec

**Status**: Phase B (Dirty Propagation Finalization) — **COMPLETE**  
**Architecture Lock Level**: 98% (Compiler-Grade Forensic Core)  
**Core Objective**: High-fidelity incremental structural analysis with $O(depth)$ recompute latency.

---

# 0. Architecture Evolution

This section documents the transition from legacy modeling to the v1.2.4 Identity Physics.

## v1.2.3 (Legacy Model: Text-Based)
- **Identity Model**: `FileHash = SHA256(AST Text)`
- **Weaknesses**:
    - **Rename Cascade**: Renaming a single local variable invalidated the entire file hash.
    - **Coarse Incrementality**: Re-parsed everything if anything changed.
    - **Latency**: $O(N)$ where $N$ is file size.
- **Core Limitation**: Text-based identity led to structural instability and high IDE lag.

## v1.2.4 (Structural Identity Model: Physics-Based)
- **Identity Model**: `NodeId = Hash(Domain, Version, Parent, Role, Anchor)`
- **Breakthroughs**:
    - **Parent-Anchored Identity**: Deterministic stability regardless of text shifts.
    - **Rename-Safe**: Local renames do not propagate to parents.
    - **Performance**: $O(depth)$ latency for deep edits.
- **Impact**: Zero-lag IDE integration and forensic-grade structural drift detection.

---

# 1. System Architecture Map

Definition of module boundaries to prevent logic misplacement and maintain system sterility.

### `vantage-types/` (Core Physics)
- **Files**: `node_id.rs`, `identity_anchor.rs`, `semantic_role.rs`, `caf.rs`, `semantic.rs`.
- **Responsibilities**:
    - Absolute Definition of Identity (Identity Physics).
    - Immutable Structural Representation (CAF Nodes/Hashes).
    - Scope Context Modeling.

### `vantage-core/` (Runtime Engine)
- **Files**: `parser/`, `dirty_propagation.rs`, `arena.rs`, `telemetry.rs`.
- **Responsibilities**:
    - AST Traversal & Orphan Management.
    - $O(depth)$ Dirty Propagation Logic.
    - Persistence & Memoization (NodeArena).

### `vantage-cli/` (Orchestration)
- **Files**: `main.rs`, `dispatch.rs`, `term.rs`.
- **Responsibilities**:
    - Benchmark Execution & CLI Interface.
    - High-level Workflow Dispatching.

---

# 2. Core Identity Physics

The identity of any structural node is determined by its **Identity Physics**, ensuring that a node's ID remains stable across renames, moves (if commutativity allows), and re-parses.

### NodeId Formula
```rust
NodeId = Hash(
    Domain,         // Logical workspace/project domain
    Version,        // HashVersion (ABI compatibility)
    Parent,         // NodeId of the logical parent
    Role,           // Semantic role (e.g., Binding, Body, Parameter)
    Anchor          // IdentityAnchor (Structural or Binding/SymbolId)
)
```

## Invariants
1. **parent_id is immutable**: Once a node is inserted into the `NodeArena`, its parent relationship cannot change.
2. **Role numbering is ABI-stable**: `SemanticRole` enums must preserve ordering to ensure hash stability.
3. **Anchor serialization is deterministic**: `IdentityAnchor` uses lexicographical sorting or stable discovery IDs.
4. **DOMAIN_ROOT is constant**: The tree is anchored to a global root ID to prevent drift.
5. **NodeId must never be zero**: Reserved for uninitialized or invalid states.

---

# 2. Scope Invariance Model

Vantage enforces **Scope Invariance** to ensure that renames within a scope do not cause a Merkle hash cascade.

## 2.1 HashVersion Lifecycle
The `HashVersion` (located in `vantage-types/src/version.rs`) MUST be bumped when:
1. **SemanticRole enum changes**: Adding or reordering roles changes the hash salt.
2. **IdentityAnchor layout changes**: Any change to how anchors are serialized.
3. **NodeId hashing order changes**: Modifying the raw bytes sequence sent to Sha256.
4. **Scope resolution rules change**: Any logic shift in `AlgebraResolver` or `CafContext`.

> [!IMPORTANT]
> **Effect**: A version bump safely invalidates all existing caches across the workspace. Failure to bump version on breaking identity changes leads to persistent, non-deterministic cache corruption.

## Rule
**ScopeContext MUST NOT be recreated per node.**

- **Correct**: `CafBuilder` derives `ScopeContext` from the persistent `CafContext` stack.
- **Logic**: Commutativity resolution (e.g., "are these statements order-independent?") depends on the actual structural depth and loop/match/macro nesting level provided by the `CafContext`.

---

# 3. Dirty Propagation Model

## Complexity Goal: $O(depth)$
Latency for incremental updates must be proportional to the depth of the edit, not the total size of the file.

## Algorithm: Iterative Upward Walk
1. Start at `leaf_id`.
2. While `current_node.parent_id` exists:
    - Mark `current_node` as `dirty`.
    - If `parent_node.dirty == true` -> **Short-circuit** (already propagated).
    - Move to `parent_id`.

## Safety Guard
- **MAX_DEPTH**: 4096 (prevents infinite loops in cyclic or corrupted graphs).

---

# 4. Parser Execution Model

## Strategy: Child-First Recursion
- Parent `CafHash` depends on the hashes of its children.
- Recursive walk ensures that child identities and structural hashes are ready before the parent is finalized.

## Memoization (NodeArena)
- **Lookup**: $O(1)$ via `HashMap<NodeId, NodeEntry>`.
- **Generation Bumping**: Reused nodes must have their `generation` bumped to survive the local Garbage Collection (GC) at the end of each pass.

## 4.1 Arena Lifetime Model

### NodeArena Lifetime
- **Persistent**: The arena persists as long as the `EpistemicParser` session (or daemon) is alive.
- **Sterile Reset**: Cleared/Re-initialized ONLY on:
    - Manual workspace reload.
    - `HashVersion` bump detection.
    - Systematic memory pressure trigger.

### Generation-Based GC
- Each `parse_signals` call increments the internal `generation` counter.
- During the walk, hits call `get_mut_with_bump(current_gen)`.
- At the end of the walk, `arena.gc()` evicts any `NodeEntry` where `generation < current_gen`.
- **Effect**: Orphan nodes from deleted code are reclaimed in $O(N_{dead})$ time.

---

# 5. Performance Targets

| Metric | Target | Verified (v1.2.4 Phase B) |
| :--- | :--- | :--- |
| **Identical Parse** | 100% Reuse | **100.00%** |
| **Deep Edit** | $O(depth)$ Recompute | **Verified** |
| **Warm Latency** | < 1ms (1k lines) | **~330 µs** |

---

# 6. Phase Roadmap

- [x] **Phase A — Identity Stabilization**: ABI-stable `SemanticRole` and deterministic `NodeId`.
- [x] **Phase B — Dirty Propagation**: Hardened iterative traversal and Scope Invariance fix.
- [ ] **Phase C — Dependency Graph**: Propagating structural changes to symbol relationships.
- [ ] **Phase D — Budget Control**: 12ms enforcement and resource capping.
- [ ] **Phase E — Snapshot**: Forensic sealing and persistence.

---

# 7. Death Tests Contract

The following guarantees are verified by the internal test suite and `proof_o_depth.rs`:

1. **test_identity_stability**: Renaming a variable MUST NOT change the `NodeId` of its parent.
2. **test_dirty_short_circuit**: Dirty propagation MUST stop at the first already-dirty parent.
3. **test_scope_leak**: `CafContext` transitions (push/pop) must be perfectly balanced during AST traversal.

---

# 8. Known Architectural Risks (Resolved)
- **Problem**: `ScopeContext` reset per build.
- **Fix**: Persistent `CafContext` management in `EpistemicParser` orchestration.

---

# 9. Glossary

- **NodeId**: 128-bit deterministic structural identity.
- **CafHash**: Merkle hash representing the structural content of a node.
- **Anchor**: Stable identity seed (Structural or Binding).
- **Role**: The semantic relationship of a node to its parent.
- **Dirty**: A flag indicating a node's `CafHash` needs recomputation.

---

# 10. Dependency Graph Contract (Phase C Bridge)

**Objective**: Propagate structural changes and identity shifts to dependent symbols.

## Phase C Data Model (Step 0)
The dependency graph is a directed graph where nodes represent symbols and edges represent structural/semantic dependencies.

```rust
pub struct SymbolDependencyGraph {
    /// Maps each symbol to its dependency metadata
    pub nodes: HashMap<SymbolId, DepNode>,
}

pub struct DepNode {
    pub symbol: SymbolId,
    /// What this symbol DEPENDS ON (Outgoing edges)
    pub dependencies: HashSet<DependencyEdge>,
    /// What DEPENDS ON this symbol (Incoming edges)
    pub dependents: HashSet<SymbolId>,
}
```

## 10.1 Symbol Canonicalization Invariant
To handle re-exports and alias drift, `SymbolId` MUST be canonicalized to its **Fully Qualified Name (FQN)** after resolution.
- **Rule**: `crate::module_b::foo` -> `crate::module_a::foo` if `module_b` re-exports from `a`.
- **Effect**: Prevents duplicate nodes and "Ghost Edges" in the graph.

## 10.2 SymbolHash Invariant
To control the blast radius of incremental re-analysis, each symbol tracked in Phase C MUST have a bipartite hash:
```rust
pub struct SymbolHash {
    pub signature_hash: CafHash, // Affects dependents
    pub body_hash: CafHash,      // Only affects local validation
}
```
- **Invariant**: A change in `body_hash` alone MUST NOT mark `dependents` as Dirty.

## 10.3 Tarjan Trigger Rule
Cycle detection is computationally expensive.
- **Constraint**: Tarjan's SCC algorithm MUST be triggered ONLY when an edge is **Added** or **Removed**.

## 10.4 SymbolId Stability Boundary
Once Phase C Step 1 (Integration) begins, the `SymbolId` format (canonical FQN) MUST remain stable.
- **Rule**: Any change to the FQN schema or interning model requires a mandatory version bump and full workspace re-index.
- **Effect**: Prevents silent graph corruption during future refactors.

## 10.5 Forensic Edge Metadata (Bipartite Connectivity)
Every dependency edge in the graph MUST be traceable and forensic-ready:
```rust
pub struct DependencyEdge {
    pub source: SymbolId,        // The symbol containing the reference
    pub target: SymbolId,        // The symbol being referred to
    pub kind: DependencyKind,    // Semantic nature of the link
    pub span: Option<SourceSpan>, // Byte offsets for IDE precision
}
```

## 10.6 Internal Hash Caching (Performance Stability)
To prevent "Recompute Storms", the current `SymbolHash` MUST be cached directly within each `DepNode`.
- **Constraint**: Validation only triggers a hash recalculation if the underlying `NodeId` is marked Dirty.

## 10.7 Dirty Queue & Reverse Propagation
Phase C implements an explicit **Dirty Queue** (`VecDeque<SymbolId>`) for efficient reverse dependency traversal.
- **Strategy**: When a node is marked Dirty, its **dependents** are pushed to the queue for deferred validation.
- **Complexity**: $O(E_{dirty})$ where $E$ is the number of affected edges.

## Core Rule
If the `NodeId` or `SymbolHash::signature_hash` of a symbol changes, the engine MUST identify all **dependents** via the graph and mark them as **Dirty**.

## Cycle Handling & Propagation
- **Invariant**: The dependency graph may contain cycles (e.g., recursive calls). 
- **Constraint**: **Tarjan's Strongly Connected Components (SCC)** algorithm MUST be used to detect cycles and prevent stack overflow during propagation.

---

# 11. Glossary

- **NodeId**: 128-bit deterministic structural identity.
- **CafHash**: Merkle hash representing the structural content of a node (content-invariant).
- **SymbolHash**: Bipartite hash (Signature vs Body) used for blast-radius control.
- **SymbolId**: FQN-based canonical identifier for a named structural entity.
- **Anchor**: Stable identity seed (Structural or Binding).
- **Role**: The semantic relationship of a node to its parent.
- **Dirty**: A flag indicating a node's `CafHash` or its semantic dependencies need recomputation.
- **Generation**: Parse pass counter used for Arena Garbage Collection.
- **Tombstoned**: A symbol state indicating exclusion from the current AST (awaits dependent invalidation).
- **Bipartite Identity**: Separation of physical location (`NodeId`) from logical essence (`SymbolId`).

---

# 12. Symbol Lifecycle Model (Forensic-Grade State Machine)

To prevent ghost dependencies and dangling symbols, every `SymbolId` strictly adheres to the following State Machine during the parsing lifecycle:

## Symbol States
1. **Discovered**: The symbol's signature is parsed, but its references and body are not yet analyzed.
2. **Bound**: The symbol has been successfully mapped to its AST `NodeId`.
3. **Validated**: All outgoing dependencies (what it uses) and incoming dependencies (who uses it) are verified and hashed.
4. **Dirty**: The underlying `NodeId` structure changed OR an incoming `SignatureRef` dependency changed. Awaiting recomputation.
5. **Tombstoned (Deleted)**: The symbol is no longer found in the AST. 

## 12.1 Tombstone Graceful Eviction
To prevent dependency drift during rapid edits:
- **Rule**: A Tombstoned symbol MUST be retained in the graph for **exactly one generation** to broadcast invalidation before removal.

---

# 13. Cross-File & Move Semantics (Blast Radius Control)

To prevent "Global Recompute Storms" (where changing one file lags the whole workspace), Phase C implements strict Cross-File and Move handling rules.

## 13.1 Bipartite Identity (Move Stability)
Vantage uses a dual-identity system to survive structural refactoring:
- **Structural Identity (`NodeId`)**: Where the node lives in the file tree (Parent-anchored). Changes on Move.
- **Logical Identity (`SymbolId`)**: The semantic essence of the symbol (e.g., `crate::module::FunctionA`). 
- **The Move Invariant**: If a function is moved to a new file, its `NodeId` changes, but if its `SymbolId` remains stable (via re-exports or unchanged FQN), the `SymbolDependencyGraph` edges remain INTACT. Downstream consumers are NOT marked dirty.

## 13.2 Dependency Edge Granularity
When connecting `SymbolId` A to `SymbolId` B, the edge MUST specify a `DependencyKind` to control the invalidation blast radius:

1. **`SignatureRef` (Strict)**: 
   - Example: File A uses `Struct B` as a return type.
   - Behavior: If `B` changes its structure, `A` MUST be marked Dirty.
2. **`CallEdge` (Loose)**: 
   - Example: File A calls `Function B`.
   - Behavior: If `B` changes its INTERNAL body (`NodeId` changes but Signature hash is identical), `A` is NOT marked dirty.
3. **`ModuleImport` / `ReExport`**:
   - Behavior: Only changes visibility scope. If `File B` removes a `pub`, dependents in `File A` are Tombstoned if they cannot find an alternative path.

---

# 14. System Role Contract (Vantage–Kit–Agent Separation)

## 14.1 Role Hierarchy
```text id="role0"
L0: Vantage (Rust Sensor Engine)
L1: Kit (Python Orchestrator 3.14.3)
L2: Agent (Execution Layer)
```

## 14.2 Vantage Role (STRICT DEFINITION)

### Vantage IS:
- deterministic structural sensor
- identity + graph computation engine
- stateless decision-free system
- snapshot generator of program structure

### Vantage IS NOT:
- ❌ planner
- ❌ orchestrator
- ❌ reasoning engine
- ❌ workflow controller
- ❌ task scheduler

### Vantage OUTPUT CONTRACT:
```text id="vout1"
ONLY ALLOWED OUTPUT TYPES:

- SymbolId graph snapshots
- DependencyGraph diff
- Dirty propagation results
- Identity resolution results
- Structural events
```

## 14.3 Kit Role (CRITICAL CONTROL LAYER)

### Kit IS:
- system brain / orchestrator
- decision maker
- planner
- agent controller

### Kit responsibilities:
- decide WHAT to ask Vantage
- decide WHEN to query graph
- merge multiple Vantage snapshots
- maintain global reasoning state
- generate execution plan for Agent

### Kit IS NOT:
- ❌ structural engine
- ❌ identity resolver
- ❌ graph builder

## 14.4 Agent Role
- pure executor
- no structural awareness
- no graph access
- only consumes Kit instructions

## 14.5 Direction of Authority (VERY IMPORTANT)
```text id="flow0"
Agent → Kit → Vantage
```
BUT:
```text id="flow1"
Vantage NEVER initiates communication
Vantage NEVER triggers workflows
Vantage NEVER decides analysis scope
```

## 14.6 Data Flow Contract

### Kit → Vantage (REQUEST ONLY)
Allowed:
- analyze(file)
- diff(symbol)
- query(dependencies)
- apply_change(event)

NOT allowed:
- "plan next steps"
- "optimize strategy"
- "decide what to analyze"

### Vantage → Kit (RESPONSE ONLY)
Allowed:
- graph snapshot
- dependency diff
- identity resolution
- structural mutation result

## 14.7 Source of Truth Rule
```text id="truth0"
Vantage = SOURCE OF STRUCTURAL TRUTH
Kit = SOURCE OF EXECUTION TRUTH
Agent = SOURCE OF ACTION TRUTH
```
BUT:
- Vantage truth is **non-semantic**
- Kit truth is **semantic interpretation**

## 14.8 Critical Invariant (SYSTEM SAFETY)
```text id="inv0"
Vantage MUST NOT depend on Kit state
Kit MAY depend on Vantage state
Agent MUST NOT depend on Vantage directly
```

---

# 15. Constitutional Invariants & Forensic Determinism (v1.2.4-VM)

To transition from an engineering prototype to a **Deterministic Structural Virtual Machine**, Vantage enforces the following forensic invariants.

## 15.1 ABI Snapshot Seal (Schema Lock)
The system's binary interface is locked via a `SYSTEM_ABI_HASH`. This hash covers:
- `SymbolId` binary layout (index + epoch).
- `SymbolRegistry` state schema.
- `Protocol V0` DTO structures.
**Rule**: Any change to these structures MUST result in a `SYSTEM_ABI_HASH` mismatch, triggering a mandatory forensic invalidation.

## 15.2 Deterministic Replay Equivalent
Replay identity is not just "bit-for-bit" equality, but **Canonical Structural Equivalence**.
- **Equivalence Rule**: Two graphs are identical IF:
    - Their node sets are identical (SymbolId comparison).
    - Their edge sets are identical after **lexicographical sorting** of FQNs.
    - Their internal `SymbolHash` values match for all nodes.
- **Constraint**: Serialization MUST enforce deterministic iteration order of HashMaps (e.g., sorting by `SymbolId.index` during emission).

## 15.3 Dual-Lens Consistency (Semantic Proof)
The system must prove that its numeric optimization does not diverge from semantic reality.
- **Invariant**: `Graph(SymbolId space) == normalize(Graph(FQN space))`.
- **Proof**: Resolving every numeric `SymbolId` back to its FQN string and rebuilding the graph MUST yield a structure isomorphic to the original.

## 15.4 Stability under Non-Determinism
The engine's output MUST be invariant to runtime non-determinism, including:
- Random insertion order into the `SymbolRegistry`.
- HashMap reordering/rehashing.
- Concurrent interning race conditions.
**Requirement**: The `Vantage Invariant Test Harness v1` MUST force these perturbations during a "Stress Replay" to verify output bit-identity.

