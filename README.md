# Vantage (v1.2.4)

> **Stateless Structural Code Sensor** for Cognitive Architectures.

Vantage is a high-performance Rust binary designed to provide deterministic structural identity for source code. It is a "Cognitive Sensor" that maps raw bytes to stable epistemic signals, enabling AI agents and CI systems to detect drift with 100% precision.

---

### 🛡️ System Integrity
- **Stateless**: No memory, no background process.
- **Deterministic**: Same source → same identity everywhere.
- **Sealed**: v1.2.4 — Locked for production.

---

### 📖 Master Contract
For all CLI commands, JSON schemas, Agent protocols, and Technical specifications, see:
👉 **[VANTAGE_CONTRACT.md](file:///VANTAGE_CONTRACT.md)**

---

### 🧠 Cognitive Layer
For Agent protocols, constitutional invariants, and Kit syntax, see:
👉 **[kit/AGENTS.md](file:///kit/AGENTS.md)**

---

### 🚀 Quick Start
```bash
# Verify a file
vantage verify src/main.rs --json

# Check for drift
vantage diff core/utils.rs

# Create a forensic baseline
vantage seal .
```

---

*v1.2.4 — Knowledge Router Operational*
