# Spec: Unified Symbol Schema (v1.0)
Timestamp: 2026-03-24

## 1. The CognitiveSignal (L2 Output)
This is the standard payload emitted by Vantage sensors to be ingested by the Kit brain.

```rust
pub struct CognitiveSignal {
    pub uuid: String,            // Epistemic identifier (@epistemic:<uuid>)
    pub symbol_id: String,       // Stable identity (e.g., "core::parser::solve")
    pub parent: Option<String>,  // Hierarchy parent UUID or name
    pub symbol_kind: SymbolKind, // function, class, struct, etc.
    pub language: String,        // "rust", "python", "markdown"
    pub structural_hash: String, // AST shape hash (Sha256)
    pub semantic_hash: String,   // Behavioral hash (stripping noise)
    pub signature: Option<String>,
    pub metadata: HashMap<String, String>,
    pub origin: Origin,          // Source parser details
    pub confidence: f32,         // 0.0 -> 1.0
}

pub struct Origin {
    pub parser: String,          // e.g., "tree-sitter-rust"
    pub version: String,         // e.g., "0.23.0"
}
```

## 2. Confidence Rules
- **1.0 (Exact):** Direct AST match with 100% deterministic mapping.
- **0.8 (Inferred):** Symbol matched via proximity or partial naming in the context.
- **0.5 (Heuristic):** Statistical guess or broad pattern match (e.g., regex fallback).
- **<0.5 (Unreliable):** High-noise data, only used for research purposes.

## 3. Hashing Strategy
- **Structural Hash:** SHA256 of the normalized AST byte range.
- **Semantic Hash:** SHA256 of the "Solid" content node only (stripping whitespace, comments, and non-logical nodes).
