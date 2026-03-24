# Vantage v1.2.3 🛡️

**Vantage** is a high-precision **Structural Code Sensor** designed for the `kit` ecosystem. It detects and prevents structural drift in codebase evolution by enforcing deterministic physical and semantic hashes.

## 🚀 Key Features
- **AST-Based Fingerprinting**: Language-specific parsing (Rust, Python) for deep structural analysis.
- **Epistemic Anchors**: Link code blocks to immutable identity via `@epistemic:<uuid>` tags.
- **Zero Silent Failure**: Guaranteed detection of structural changes, even if logic appears similar.
- **Sealed & Stateless**: Single-shot CLI for deterministic CI/CD and Agentic workflows.

## 🛠️ Usage

### Verify a file
```bash
./vantage-verify path/to/file.rs --json
```

### Response (Contract)
Vantage outputs a `StructuralSignal` following the schema defined in `specs/signal_schema.md`.

## 📦 Integration
Vantage is a first-class plugin for the [kit](https://github.com/anomalyco/kit) ecosystem.

---
*Vantage: Because code drift is the enemy of truth.*