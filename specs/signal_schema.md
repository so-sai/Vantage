# Unified Signal Schema (v1.2.3)

The Unified Signal Schema is the standard contract for structural abstraction across all supported languages (Rust, Python).

---

## 📡 1. Signal Definition: `StructuralSignal`

| Field | Type | Description |
| :--- | :--- | :--- |
| `uuid` | `string` | Epistemic identifier extracted from `@epistemic:<uuid>` |
| `symbol_id` | `string` | Stable identifier (e.g., `vantage::parser::solve`) |
| `symbol_kind` | `enum` | Type: `Function`, `Class`, `Struct`, `Trait`, `Module`, `Other` |
| `language` | `enum` | Source language: `rs`, `py` |
| `structural_hash` | `sha256` | Byte-level structural hash (shape-based) |
| `normalized_hash` | `sha256` | Whitespace/Comment invariant hash (formerly semantic_hash) |
| `location` | `object` | Geometric target (file, start_line, end_line) |
| `signature` | `string` | Language-specific logical signature (Option) |

---

## 📡 2. Integrity Principle: The "Double Lock"
Each signal carries two cryptographic hashes to ensure forensic integrity:

1. **Structural Lock (`structural_hash`)**: Detects any physical change in the code block.
2. **Normalized Lock (`normalized_hash`)**: Detects functional shifts while ignoring formatting (whitespace, comments).

---

## 📡 3. CLI Contract (`cli_output.json`)
All successful sensor executions must return a JSON object following the [contracts/cli_output.json](file:///e:/DEV/opensource_contrib/Vantage/contracts/cli_output.json) schema.

### Explicit Outcome Pattern
Wait-free verification requires explicit outcomes:
- `ok`: Signals extracted successfully.
- `error`: Parser failure or configuration error.
- `ok` + `reason: no_anchor_found`: Successful parse, but no `@epistemic` markers detected.
