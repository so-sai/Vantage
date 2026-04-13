use crate::introspection::{CapabilityDescriptor, VantageCapabilityRegistry};

static CAPABILITY_ENTRIES: &[CapabilityDescriptor] = &[
    CapabilityDescriptor {
        name: "dirty_propagation",
        inputs: &["NodeId"],
        outputs: &["DirtySet"],
        invariants: &["O(depth)", "no global state", "short-circuit safe"],
        help: "Propagates dirty status from leaf nodes up to root. Short-circuits on first dirty node encountered.",
    },
    CapabilityDescriptor {
        name: "symbol_dependency_graph",
        inputs: &["SourceCode", "Language"],
        outputs: &["SymbolGraphDTO"],
        invariants: &["deterministic", "bipartite identity"],
        help: "Extracts symbol dependency graph from source code using Bipartite Identity (NodeId/SymbolId).",
    },
    CapabilityDescriptor {
        name: "incremental_parse",
        inputs: &["SourceCode", "PreviousState"],
        outputs: &["IncrementalState"],
        invariants: &["O(depth) on edit", "cache preservation"],
        help: "Incremental AST parsing with dirty region tracking and cache reuse.",
    },
    CapabilityDescriptor {
        name: "triple_hash",
        inputs: &["SourceCode"],
        outputs: &["StructuralHash", "SemanticHash", "NormalizedHash"],
        invariants: &["CRLF normalization", "deterministic"],
        help: "Triple hash system: structural (any change), semantic (whitespace invariant), normalized (rename invariant).",
    },
    CapabilityDescriptor {
        name: "drift_detection",
        inputs: &["BaselineSignals", "CurrentSignals"],
        outputs: &["DriftReport"],
        invariants: &["normalized_hash alignment", "false negative prevention"],
        help: "Detects structural and semantic drift between baseline and current signals.",
    },
    CapabilityDescriptor {
        name: "invariant_enforcement",
        inputs: &["CognitiveSignal", "InvariantRule"],
        outputs: &["Verdict"],
        invariants: &["deterministic", "no false positives"],
        help: "Enforces cognitive invariants on parsed signals using claim → verdict pipeline.",
    },
    CapabilityDescriptor {
        name: "tombstone_eviction",
        inputs: &["DepNode"],
        outputs: &["Generation"],
        invariants: &["one-generation retention", "invalidation broadcast"],
        help: "Graceful eviction: tombstones retained for one generation before automatic removal.",
    },
    CapabilityDescriptor {
        name: "blast_radius_control",
        inputs: &["SymbolHash"],
        outputs: &["SignatureHash", "BodyHash"],
        invariants: &["independent hashing", "sub-millisecond re-analysis"],
        help: "Splits hash into signature (affects dependents) and body (local only) for incremental re-analysis.",
    },
];

pub static CAPABILITY_REGISTRY: VantageCapabilityRegistry = VantageCapabilityRegistry {
    entries: CAPABILITY_ENTRIES,
};
