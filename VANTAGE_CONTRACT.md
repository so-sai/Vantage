# VANTAGE MASTER CONTRACT (v1.2.4)

> **CANONICAL AUTHORITY** for all structural sensors, agents, and CI interactions.
> Vantage is a **stateless structural sensor** providing deterministic code identity.

---

## 1. PURPOSE & IDENTITY
Vantage maps physical source code (L0) to structural symbols (L2) using a triple-hash identity system. It does not interpret intent; it only verifies structure and relationships.

---

## 2. CLI INTERFACE
Vantage is invoked as a single-shot binary with no runtime dependencies.

```bash
vantage <command> [args] [--json]
```

### Commands:
- `verify <file> [--enforce (EXP)]`: Parse source, extract signals, run pipeline.
- `graph <file> (EXP)`: Extract dependency edges (calls, imports).
- `diff <file> [--seal .] (EXP)`: Compare against `VANTAGE.SEAL` baseline.
- `seal <path> (EXP)`: Create forensic baseline for a directory.
- `purge --force (EXP)`: Remove local forensic artifacts.

> [!CAUTION]
> **EXPERIMENTAL MODE ACTIVE (v1.2.4-EXPERIMENTAL)**
> Modules marked with `(EXP)` are operational but unverified on large-scale repositories. AI Agents must treat outputs as forensic evidence requiring high-level reasoning.

---

## 3. OUTPUT MODEL (JSON)
Every JSON output root contains `"v": "1.2.4"` and a `"status"` field.

### CognitiveSignal Schema:
| Field | Type | Description |
| :--- | :--- | :--- |
| `id` | `uuid` | Stable Epistemic identifier from `@epistemic:<uuid>` |
| `name` | `string` | Symbol name (e.g., `calculate_total`) |
| `kind` | `enum` | `Function`, `Class`, `Struct`, `Trait`, `Module`, etc. |
| `structural_hash` | `sha256` | Byte-level identity (detects any change) |
| `semantic_hash` | `sha256` | Whitespace-invariant identity |
| `normalized_hash` | `sha256` | Rename-invariant identity (AST S-expression) |
| `location` | `object` | `{ file, start_line, start_col, end_line, end_col }` |

### FailureReason Taxonomy:
If `status` is `error`, `reason` will be one of:
- `syntax_error`: Source cannot be parsed.
- `unsupported_language`: No grammar for file extension.
- `no_anchor_found`: No `@epistemic` tag detected.
- `file_read_error`: I/O failure.
- `internal_error`: Software failure.

---

## 4. SYSTEM INVARIANTS (NON-NEGOTIABLE)

1. **Determinism**: Same source → identical JSON output across all OS/environments.
2. **Stateless**: No internal memory, no daemon, no background process.
3. **Triple-Hash Matrix**: 
   - Whitespace change → `structural` changes, others same.
   - Variable rename → `structural`/`semantic` change, `normalized` same.
   - Logic change → ALL hashes change.
4. **Symbol Graph**: Extracts `Calls`, `Imports`, and `Uses` edges for impact radius analysis.
5. **Limitations**: Breaks on Unicode control chars; no stdin/stdout piping; file-only input.

---

## 5. AGENT PROTOCOL (FOR AI AGENTS)

AI Agents (like Antigravity) MUST adhere to these protocols when interacting with Vantage:

### Boundary Rules:
- **Never modify** files outside the project root.
- **Never edit** dependencies or external toolchains.

### Safety Layer:
1. **Lazy Hydration**: Do NOT read raw file content by default. Call `vantage verify` first.
2. **Structural Guard**: If a file has `@epistemic` markers, treat it as "Locked". Any `normalized_hash` change must be flagged for impact radius review.
3. **Impact Radius**: Use `vantage graph` to identify affected callers BEFORE modifying a function.

### Prohibitions:
- **No server/daemon**: Do not attempt to run Vantage as a persistent service.
- **No inference**: Do not "guess" the meaning of hashes; use the `diff` command for drift reporting.

---

---

## 6. EXPERIMENTAL MODULES & AI TESTING

### Experimental Status (v1.2.4)
- **Verify**: STABLE. Core structural extraction is locked.
- **Graph/Diff/Seal**: EXPERIMENTAL. These modules are open for AI testing in v1.2.4 to gather friction data for the next release.

### AI Reasoning Protocol (v1.2.4-EXPERIMENTAL)
1. **Evidence over Truth**: AI agents must not assume a `vantage` output is the final "Ground Truth". Use it as a structural signal to inform your next research step.
2. **False Positives**: Be prepared for noise in `diff` and `graph` outputs. If a signal contradicts your source code reading, log the friction using `kit learn --kind note --tag decision`.

---

## 7. ROADMAP (v1.2.5)

1. **Deterministic Normalization**: Hardening AST S-expression mapping for a wider range of Rust/Python patterns.
2. **Global Anchor Resolution**: Moving beyond local file analysis to resolve `@epistemic` anchors across the entire workspace.
3. **Forensic Report UI**: Implementing the `vantage report` command for human-readable structural summaries.

---

*v1.2.4 — EXPERIMENTAL MODE ENABLED. Single Source of Truth Under Stress Test.*
