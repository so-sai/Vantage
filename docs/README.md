# Vantage (v1.2.5)

> **Stateless Structural Code Sensor** for Cognitive Architectures.

Vantage is a high-performance Rust binary designed to provide deterministic structural identity for source code. It is a "Cognitive Sensor" that maps raw bytes to stable epistemic signals, enabling AI agents and CI systems to detect drift with 100% precision.

---

### 🛡️ System Integrity
- **Stateless**: No memory, no background process.
- **Deterministic**: Same source → same identity everywhere.
- **Sealed**: v1.2.5 — Locked for production.

---

### 📖 Master Contract
For all CLI commands, JSON schemas, Agent protocols, and Technical specifications, see:
👉 **[docs/VANTAGE_CONTRACT.md](file:///docs/VANTAGE_CONTRACT.md)**

---

### 🧠 Cognitive Layer
For Agent protocols, constitutional invariants, and Kit syntax, see:
👉 **[kit/AGENTS.md](file:///kit/AGENTS.md)**

---

### 🚀 Quick Start
```bash
# Verify a file
kit-vantage verify src/main.rs --json

# Check for drift
kit-vantage graph core/utils.rs

# Create a forensic baseline
kit-vantage run .
```

## Deterministic Guarantees
- No AI inference
- No probabilistic heuristics
- No runtime coupling
- Cross-language stable graphing
- Canonical UTF-8 serialization
- Offline reproducibility
- Git-native verification
- **v1.2.5 Schema Immutability Locked**
- **Deterministic Canonical Output Enforced**

## Master Contract Summary
| Layer | Role | Authority |
| :--- | :--- | :--- |
| **Vantage** | Structural Sensor | Code Identity (L2) |
| **Kit** | Cognitive Brain | Intent & Memory |
| **Git** | Time Kernel | Revision History |

---

*v1.2.5 — Knowledge Router Operational*
