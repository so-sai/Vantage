# RELEASE_MANIFEST.md

**Version**: 1.2.3  
**Codename**: Structural Abstraction  
**Epoch**: 2026-03-24  

## 🛡️ Guarantees
- **Deterministic Hashing**: Identical AST inputs produce bit-identical structural hashes.
- **Normalization**: Semantic hashes ignore whitespace and comment-only changes.
- **Statelessness**: No side-effects, no background processes, no cache dependency.

## 🎯 Scope
- **Languages**: Rust (Edition 2021/2024), Python (3.10+).
- **Interface**: CLI (Single-shot execution).
- **Environment**: Cross-platform (Windows, Linux, macOS).

## 🚫 Non-Goals
- No persistent memory or background monitoring.
- No network access or external telemetry.
- No semantic inference beyond AST structure.

---
*This manifest seals the v1.2.3 architectural boundary.*
