# VANTAGE CLI CONTRACT v1.2.4

> **Canonical authority** for how any system (Kit, IDE, CI) invokes Vantage.
> Vantage is a **stateless structural sensor** — not a library, not a daemon.
> Kit calls Vantage the same way it calls `git`.

---

## Invocation

```bash
vantage <command> [args] [--json]
```

- **Binary**: single executable, no runtime dependencies
- **State**: none — every invocation is independent
- **Stdout**: human-readable by default, JSON with `--json`
- **Stderr**: nothing on success, error details on failure
- **Exit codes**: `0` = success, `1` = failure

---

## Commands

### `verify <path> [--json] [--enforce]`

Parse a source file, extract epistemic signals, run pipeline.

**Success output (`--json`):**

```json
{
  "v": "1.2.4",
  "status": "ok",
  "file": "src/main.rs",
  "language": "rs",
  "signals": 3,
  "claims": 5,
  "verdicts": [
    {
      "decision": "allow",
      "claim_id": "uuid::FunctionDefinition",
      "reason": "no_rule_matched"
    }
  ],
  "final_decision": "allow",
  "duration_ms": 2
}
```

**Error output (`--json`):**

```json
{
  "v": "1.2.4",
  "status": "error",
  "reason": "file_read_error",
  "message": "The system cannot find the file specified.",
  "file": "nonexistent.rs"
}
```

**FailureReason taxonomy:**

| Enum variant | JSON value | When |
|---|---|---|
| `SyntaxError` | `syntax_error` | Source cannot be parsed as valid AST |
| `UnsupportedLanguage` | `unsupported_language` | File extension has no grammar |
| `NoAnchorFound` | `no_anchor_found` | No `@epistemic` tag in file |
| `FileReadError` | `file_read_error` | I/O error (not found, permission) |
| `InternalError` | `internal_error` | Vantage internal failure |

**Supported languages:** `.rs` (Rust), `.py` (Python)

---

### `graph <path> [--json]`

Extract dependency graph (call edges, import edges) from source.

**Success output (`--json`):**

```json
{
  "v": "1.2.4",
  "status": "ok",
  "file": "src/main.rs",
  "signals": 5,
  "graph": {
    "nodes": 5,
    "edges": [
      {
        "from": "caller",
        "to": "callee",
        "edge_type": "calls"
      }
    ]
  }
}
```

Edges are **sorted** by `(from, to, edge_type)` for deterministic output.

---

### `diff <path> [--seal <path>] [--json]`

Compare current file against VANTAGE.SEAL baseline.

**Success output (`--json`):**

```json
{
  "v": "1.2.4",
  "total_symbols": 5,
  "unchanged": 3,
  "structural_changes": 1,
  "semantic_changes": 0,
  "added": 1,
  "removed": 0,
  "items": [
    {
      "symbol_id": "alpha",
      "status": "unchanged",
      "old_hash": "abc123...",
      "new_hash": "abc123...",
      "location": "src/main.rs:10:0"
    }
  ]
}
```

**DriftStatus values:**

| Value | Meaning |
|---|---|
| `unchanged` | structural_hash AND normalized_hash identical |
| `structural_change` | structural_hash changed, normalized_hash same (whitespace/format) |
| `semantic_change` | normalized_hash changed (logic/structure altered) |
| `added` | New symbol not in baseline |
| `removed` | Baseline symbol no longer present |

---

### `seal <path>`

Create `VANTAGE.SEAL` forensic baseline for a directory.

```json
{
  "v": "1.2.4",
  "ts": "2026-04-02T...",
  "map": [
    {
      "f": "src/main.rs",
      "s": "solve",
      "h": "a1b2c3d4..."
    }
  ]
}
```

---

### `purge --force`

Remove local forensic artifacts (`VANTAGE.SEAL`).

---

## Guarantees

### ✅ Promised

1. **Deterministic output**: Same source → same JSON on every run, every machine
2. **Stable ordering**: Signals sorted by byte position, edges sorted lexicographically
3. **Versioned contract**: Every JSON root contains `"v": "1.2.4"`
4. **No panic**: All errors return structured JSON with `status: "error"`
5. **Backward compatible**: New fields are additive; old fields never removed
6. **Zero config**: No `vantage.toml`, no environment variables required
7. **Stateless**: No daemon, no memory between invocations, no background process

### ❌ Non-goals

1. **No memory**: Vantage does not store or recall previous runs
2. **No interpretation**: Vantage does not understand business logic or intent
3. **No background process**: Vantage exits immediately after producing output
4. **No config files**: Vantage has no configuration — it is a pure sensor
5. **No plugin system**: Vantage's behavior is fixed at compile time

---

## Hash Semantics

| Hash | What it detects | Invariant to |
|---|---|---|
| `structural_hash` | ANY physical change in the symbol | Nothing — detects all changes |
| `semantic_hash` | Logic changes, identifier changes | Whitespace, formatting |
| `normalized_hash` | Structural/AST changes | Variable names, string values, integers |

**Triple hash decision matrix:**

| Scenario | structural | semantic | normalized |
|---|---|---|---|
| Whitespace reformat | Changed | Same | Same |
| Variable rename | Changed | Changed | Same |
| Logic change (`+` → `-`) | Changed | Changed | Changed |
| No change | Same | Same | Same |

---

## Cross-Platform Determinism

- Line endings: `\r\n` and `\n` produce **identical hashes**
- Encoding: UTF-8 assumed; non-UTF8 bytes normalized to spaces
- Traversal order: Pre-order AST walk, signals sorted by `byte_start`
- Edge order: Sorted by `(from, to, edge_type)`

---

## Performance Baseline

| File size | Pipeline time |
|---|---|
| Small (< 500 LOC) | < 5ms |
| Large (> 2000 LOC) | < 20ms |

Measured via `duration_ms` field in JSON output.
Process startup overhead (~300ms on Windows) is NOT included.

---

## Known Limitations (v1.2.4)

### Comment Sensitivity

Comments inside function bodies may affect `normalized_hash` due to tree-sitter parsing behavior. This can cause **false-positive drift detection** when comments are added or modified.

- **Root cause**: tree-sitter includes comment nodes in AST; anonymous node logic may capture punctuation from comments.
- **Severity**: Medium — affects trust, not correctness.
- **Planned fix**: v1.2.5 (comment filtering in normalized AST walk).
- **Kit mitigation**: Kit may discount drift signals where only comment content changed.

---

## Kit Integration Notes

Kit v1.2.4+ should:

1. Call `vantage verify <file> --json` for each file with `@epistemic` tags
2. Parse `"v"` field to determine output schema version
3. Use `FailureReason` to decide recovery strategy (skip, retry, report)
4. Monitor `duration_ms` for performance regression detection
5. Use `normalized_hash` for rename-invariant drift detection
6. Use `graph` output to compute impact radius before changing functions

---

*v1.2.4 — LOCKED. No contract changes without version bump.*
