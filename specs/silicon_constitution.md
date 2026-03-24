# Silicon Constitution (v1.2.3)

This document defines the non-negotiable invariants and boundaries for the Vantage Structural Sensor.

---

## 🛡️ I. Core Invariants (Non-Negotiable)

1. **Vantage is Stateless**: It shall not maintain any internal persistent state. Execution is always single-shot.
2. **Vantage is Non-Cognitive**: It shall not attempt to "understand" or "interpret" business logic. It strictly reports **Structural Abstractions**.
3. **Vantage is CLI-Only**: It shall not operate as a server, daemon, or background process. All interactions follow the Request-Response CLI pattern.
4. **Deterministic Hashing**: The same source code must always produce identical structural and normalized hashes across different environments.

---

## 🛡️ II. Architectural Boundaries

### 1. Host Responsibility (`kit`)
- **Cognition & Memory**: The host owns the `.kit/` directory and all historical data.
- **Orchestration**: The host decides when and how to call the sensor.
- **Drift Detection**: Comparison of current signals against historical data is a L3 (Host) responsibility.

### 2. Sensor Responsibility (`Vantage`)
- **Geometry -> Abstraction**: Mapping physical bytes (L0) to structural symbols (L2).
- **Forensic Integrity**: Providing the cryptographic proof (hashes) that a target has not drifted.
- **Explicit Failure**: Reporting specific reasons (e.g., `no_anchor_found`) instead of returning ambiguous empty results.

---

## 🛡️ III. Prohibitions (Hard Bans)

- **NO SERVER**: No MCP, HTTP, or RPC listeners.
- **NO DAEMON**: No background monitoring or auto-scans.
- **NO INFERENCE**: No LLM calls or heuristic-based "guessing" of intent.
- **NO MEMORY OWNERSHIP**: No writing to or reading from `.kit/`.
