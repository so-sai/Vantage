# Vantage Technical Specifications (v1.2.4)

**Version**: 1.2.4 | **Codename**: Knowledge Router | **Epoch**: 2026-04-02
**Languages**: Rust (Edition 2021/2024), Python (3.10+)
**Interface**: CLI (Single-shot) | **Environment**: Cross-platform

---

## 1. CognitiveSignal

| Field | Type | Description |
| :--- | :--- | :--- |
| `uuid` | `string` | Epistemic identifier from `@epistemic:<uuid>` |
| `symbol_id` | `string` | Stable identifier (e.g., `calculate_total`) |
| `symbol_kind` | `enum` | `Function`, `Class`, `Struct`, `Trait`, `Enum`, `Module`, `Other` |
| `language` | `string` | `rust` or `python` |
| `structural_hash` | `sha256` | Byte-level (any physical change detected) |
| `semantic_hash` | `sha256` | Whitespace-invariant |
| `normalized_hash` | `sha256` | Rename-invariant (identifiers stripped from AST) |
| `location` | `object` | `{ file, start_line, start_col, end_line, end_col, byte_start, byte_end }` |
| `signature` | `string?` | Language-specific logical signature |
| `confidence` | `float` | 0.0 - 1.0 |

---

## 2. Triple Hash System

| Change Type | `structural_hash` | `semantic_hash` | `normalized_hash` |
| :--- | :---: | :---: | :---: |
| Whitespace reformat | Changed | **Same** | **Same** |
| Variable rename | Changed | Changed | **Same** |
| Logic change (`+` → `-`) | Changed | Changed | Changed |

- **structural_hash**: SHA256 of raw node bytes.
- **semantic_hash**: SHA256 of bytes with all ASCII whitespace removed.
- **normalized_hash**: SHA256 of normalized AST S-expression (identifiers → `IDENT`, strings → `STR`, ints → `INT`).

---

## 3. Symbol Graph (Zero-Semantic)

Extracted during parsing from tree-sitter AST nodes.

### Edge Types

| Type | Source Node | Meaning |
| :--- | :--- | :--- |
| `Calls` | `call_expression` / `call` | A calls function B |
| `Imports` | `use_statement` / `import_statement` | A imports module B |
| `Uses` | `method_call_expression` | A uses method B |

### Impact Radius

Given a changed symbol, `impact_radius(symbol_id)` returns all symbols that **call** it.

```
main → calls → calculate
main → calls → process
main → calls → save

impact_radius("calculate") = ["main"]
```

Kit uses this to determine which functions need re-verification when a dependency changes.

---

## 4. Execution Pipeline (--enforce)

```
parse → CognitiveSignal → CognitiveClaim → InvariantEngine → Verdict
```

### Claim Types
- `FunctionDefinition` — function/async function detected
- `TypeDefinition` — struct/class detected
- `TemplateInterpolation` — f-string or format! detected
- `Structural(kind)` — generic structural claim

### Verdict Types
- **Allow**: No rule matched or rule permits.
- **Warn**: Advisory flag (e.g., template interpolation detected).
- **Reject**: Hard block, CLI exits with code 1.

---

## 5. Drift Report (vantage diff)

Compares current file state against `VANTAGE.SEAL` baseline.

| Status | Meaning |
| :--- | :--- |
| `unchanged` | All hashes match |
| `structural_change` | `structural_hash` changed, `normalized_hash` same |
| `semantic_change` | `normalized_hash` changed (real logic change) |
| `added` | New symbol not in baseline |
| `removed` | Symbol existed in baseline, gone now |

---

## 6. Architecture

```
Source Code
  ↓
tree-sitter parse (Rust / Python)
  ↓
EpistemicParser:
  ├─ find @epistemic:uuid in comments → map_to_signal (triple hash + location)
  └─ walk AST for call/import nodes → SymbolGraph (edges)
  ↓
Pipeline (--enforce): signal → claim → invariant → verdict
  ↓
Output: CognitiveSignal[] + SymbolGraph + DriftReport
```

---

## 7. Known Limitations

1. Parser breaks on Unicode control chars (`\u{000b}` vertical tab) in comments.
2. No JavaScript/TypeScript/Go/TOML support.
3. No stdin/stdout piping (file-only input).
4. Requires valid syntax (tree-sitter rejects invalid source).
5. Requires `@epistemic:<uuid>` marker in a comment adjacent to target.
6. `Calls` edge detection limited to `call_expression` nodes (excludes macros like `println!`).

---

## 8. CLI Contract

### Verify
```bash
vantage verify <file>              # human-readable output
vantage verify <file> --json       # JSON output
vantage verify <file> --enforce    # run full decision pipeline
```

### Graph
```bash
vantage graph <file>               # show dependency graph + impact radius
vantage graph <file> --json        # JSON output
```

### Diff
```bash
vantage diff <file>                # compare against VANTAGE.SEAL
vantage diff <file> --json         # JSON output
```

### Seal & Purge
```bash
vantage seal .                     # create VANTAGE.SEAL baseline
vantage purge --force              # remove local artifacts
```

### JSON Output Schema
```json
{
  "status": "ok",
  "language": "python",
  "reason": null,
  "signals": [{
    "id": "calc-v1",
    "name": "calculate_total",
    "type": "function",
    "structural_hash": "a1b2...",
    "semantic_hash": "c3d4...",
    "normalized_hash": "e5f6...",
    "location": { "file": "x.py", "start_line": 2, "start_col": 0, "end_line": 3, "end_col": 16 }
  }]
}
```

---

*v1.2.4 — Knowledge Router Operational*
