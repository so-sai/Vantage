# Vantage Agent Contract (v1.2.4)

> **CANONICAL AUTHORITY** for AI Agents. If this file conflicts with other docs, treat this as truth.

---

## Boundary Rules

- **NEVER modify files outside the current repository.**
- **NEVER edit dependencies or external toolchains (e.g., kit core).**
- If an issue is found in external systems, **REPORT it via `kit learn`** instead of fixing it.

## Token Efficiency

- **ALWAYS call `vantage verify <path>` before modifying any code.**
- **TRUST CognitiveSignal over assumptions.** Do not read entire files to guess structure.
- **IF CLI fails, STOP and report.** DO NOT fallback to guessing or raw scanning.
- **Use `normalized_hash` to detect real logic changes** — ignore `structural_hash` for refactors.
- **Use `vantage graph <path>` to understand impact radius** before changing a function.

## Role

Vantage is a stateless structural sensor with symbol graph capability.
It does not interpret meaning — only structure and relationships.
All reasoning and long-term memory must happen in the host (**kit**).

---

## Core Invariants (Non-Negotiable)

1. **Stateless**: No internal persistent state. Execution is always single-shot.
2. **Non-Cognitive**: No "understanding" or "interpretation" of business logic. Only Structural Abstractions and call relationships.
3. **CLI-Only**: No server, daemon, or background process. Request-Response pattern only.
4. **Deterministic Hashing**: Same source code → identical hashes across environments.
5. **Rename Invariance**: `normalized_hash` remains identical when only variable/function names change.

---

## Architectural Boundaries

### Host Responsibility (kit)
- Owns `.kit/` directory and all historical data.
- Decides when and how to call the sensor.
- Acts on `DriftReport` verdicts (log, warn, block).
- Uses `SymbolGraph` impact radius to determine which functions to re-verify.

### Sensor Responsibility (Vantage)
- Maps physical bytes (L0) to structural symbols (L2).
- Provides triple hash identity (structural, semantic, normalized).
- Precise source location from tree-sitter (line, column, byte offset).
- Extracts call graph (A → calls → B) for dependency analysis.
- Classifies signals into claims when `--enforce` is active.
- Reports explicit failure reasons (e.g., `no_anchor_found`).

---

## Agent Safety Layer

1. **Lazy Hydration**: Agents are forbidden from reading raw file contents by default. Call `vantage verify` first.
2. **Structural Guard**: Files with `@epistemic` markers are "Locked". Any `structural_hash` change is flagged.
3. **Rename-Invariant Identity**: `normalized_hash` ensures variable renames do NOT trigger false drift alerts.
4. **Impact Radius**: `vantage graph` shows which callers are affected by a change.
5. **No Cross-Repo Leakage**: Sensor is strictly scoped to the project root.

---

## Prohibitions (Hard Bans)

- **NO SERVER**: No MCP, HTTP, or RPC listeners.
- **NO DAEMON**: No background monitoring or auto-scans.
- **NO INFERENCE**: No LLM calls or heuristic-based "guessing" of intent.
- **NO MEMORY OWNERSHIP**: No writing to or reading from `.kit/`.

---

## Agent Workflow

```
Agent wants to modify file.rs
  ↓
vantage verify file.rs --json
  ↓
Read CognitiveSignal { normalized_hash, location }
  ↓
vantage graph file.rs
  ↓
Check impact radius: which callers are affected?
  ↓
Make changes (only affected functions)
  ↓
vantage verify file.rs --json (again)
  ↓
Compare normalized_hash:
  SAME → structural refactor, safe
  DIFF → logic changed, check impact radius
```

---

*v1.2.4 — Decision Engine + Symbol Graph Operational*
