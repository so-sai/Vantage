# Vantage – System Design

> **Cognitive Artifact Factory**: Transform documents into semantic artifacts with zero-token overhead.

**Version**: 1.0.0  
**Status**: Locked  
**Last Updated**: 2026-01-21

---

## 1. Core Principles

### 1.1 Deterministic
Every operation produces identical output given identical input. No AI inference in the critical path.

### 1.2 Zero-Token Primitives
Core operations consume no LLM tokens. AI agents orchestrate; primitives execute.

### 1.3 Intent-Driven Architecture
All interactions flow through `CognitiveIntent`. No direct function calls from UI.

### 1.4 IDE-Agnostic
No assumptions about editor, IDE, or runtime. Schema is the contract.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        ADAPTERS                              │
│  ┌─────────┐  ┌──────────┐  ┌─────────────┐  ┌───────────┐  │
│  │   CLI   │  │  VS Code │  │  Open Code  │  │Antigravity│  │
│  └────┬────┘  └────┬─────┘  └──────┬──────┘  └─────┬─────┘  │
│       │            │               │               │         │
│       └────────────┴───────┬───────┴───────────────┘         │
│                            │                                 │
│                   CognitiveIntent (v1)                       │
│                            │                                 │
└────────────────────────────┼─────────────────────────────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                      MCP SERVER                              │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │document_lens │  │ export_pdf   │  │ check_drift  │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │                │
└─────────┼─────────────────┼─────────────────┼────────────────┘
          │                 │                 │
          └─────────────────┼─────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     VANTAGE CORE                             │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                   Intent Schema v1                    │   │
│  │  • CognitiveIntent                                    │   │
│  │  • IntentKind: Lens | Export | Drift                  │   │
│  │  • IntentTarget: File | Workspace                     │   │
│  │  • IntentSource: CLI | VsCode | OpenCode | ...        │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │document_lens│  │export_to_pdf│  │    check_drift      │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Core Components

### 3.1 vantage-core (Rust Library)

The cognitive nucleus. Pure functions, no I/O side effects beyond file reading.

#### Public API

| Function | Purpose | Input | Output |
|----------|---------|-------|--------|
| `document_lens` | Semantic document analysis | File path | `DocumentData` |
| `export_to_pdf` | PDF generation | `DocumentData`, `ExportConfig` | PDF bytes |
| `check_drift` | Blueprint-code alignment | `DriftConfig` | `DriftReport` |
| `export_drift_snapshot` | Drift report as PDF | Configs | PDF bytes |

#### Intent Schema v1

```rust
pub struct CognitiveIntent {
    pub intent: IntentKind,      // What to do
    pub source: IntentSource,    // Where from (telemetry)
    pub target: IntentTarget,    // What to act on
    pub params: IntentParams,    // How (pass-through)
}

pub enum IntentKind {
    Lens,    // → document_lens
    Export,  // → export_to_pdf
    Drift,   // → check_drift
}

pub enum IntentTarget {
    File { path, selection? },
    Workspace { root },
}
```

### 3.2 vantage-mcp (MCP Server)

Model Context Protocol server exposing Core to AI agents.

#### Tools

| Tool | Intent | Description |
|------|--------|-------------|
| `document_lens` | Lens | Analyze document structure |
| `export_to_pdf` | Export | Generate PDF artifact |
| `check_drift` | Drift | Compare blueprint vs code |
| `export_drift_report` | Export + Drift | PDF snapshot of drift status |
| `check_core_status` | - | Health check |

### 3.3 vantage-cli (Reference Adapter)

CLI implementation demonstrating correct Intent usage.

#### Commands

```bash
vantage lens <file>                    # Analyze document
vantage export <file> [--mode MODE]    # Export to PDF
vantage drift <workspace> [--blueprint FILE]  # Check drift
vantage status                         # System health
```

### 3.4 IDE Thin Clients (Planned)

Minimal UI adapters that emit `CognitiveIntent` to MCP.

| Platform | Status | Implementation |
|----------|--------|----------------|
| CLI | ✅ Complete | `vantage-cli` |
| Open Code | 🔜 Planned | Intent emitter |
| Antigravity | 🔜 Planned | Intent emitter |
| VS Code | 🔜 Planned | Intent emitter |
| Claude Code | 🔜 Planned | Intent emitter |

---

## 4. Data Flow

### 4.1 Standard Flow

```
User Action (click/command)
       │
       ▼
   Adapter creates CognitiveIntent
       │
       ▼
   Intent validated (schema v1)
       │
       ▼
   MCP receives intent
       │
       ▼
   Core executes (deterministic)
       │
       ▼
   Artifact returned (PDF/Report/Data)
```

### 4.2 Export Modes

| Mode | Purpose | Use Case |
|------|---------|----------|
| `Semantic` | Preserve structure for agents | Internal processing |
| `Visual` | Clean output for humans | Client deliverables |
| `Audit` | Include signatures, timestamps | Compliance, internal review |

---

## 5. Validation Rules

### 5.1 Intent-Target Compatibility

| Intent | Required Target | Error |
|--------|-----------------|-------|
| `Lens` | `File` | `LensRequiresFile` |
| `Export` | `File` | `ExportRequiresFile` |
| `Drift` | `Workspace` | `DriftRequiresWorkspace` |

### 5.2 Path Validation

- File paths must not be empty
- Workspace roots must not be empty
- ByteSpan must have `start < end`

---

## 6. Non-Goals

### 6.1 NOT a Document Viewer
Vantage produces artifacts. Viewing is external (PDF reader, browser).

### 6.2 NOT a Rich IDE Extension
Thin Clients emit intents. No business logic in UI layer.

### 6.3 NOT an AI Model
Core is deterministic. AI agents use Core as tools.

### 6.4 NOT a Framework
Vantage is a protocol + implementation. Not extensible by design.

---

## 7. Quality Standards

### 7.1 Safe Rust 2026
- No `unwrap()` or `expect()` in production code
- All errors via `Result<T, E>` with `thiserror`
- Zero Clippy warnings

### 7.2 TDD Lock
- Core behavior locked by tests
- Intent schema locked by 15+ tests
- No regression without test update

### 7.3 Zero-Warning Policy
- `cargo clippy -- -D warnings` must pass
- All doctests must pass

---

## 8. File Structure

```
vantage/
├── DESIGN.md              # This document (Ground Truth)
├── README.md              # User-facing documentation
├── Cargo.toml             # Workspace manifest
│
├── vantage-core/          # Cognitive nucleus
│   ├── src/
│   │   ├── lib.rs         # Core functions
│   │   └── intent/        # Intent Schema v1
│   │       ├── mod.rs     # Types
│   │       └── validation.rs  # Rules + TDD
│   └── Cargo.toml
│
├── vantage-mcp/           # MCP Server
│   ├── src/
│   │   └── main.rs        # Tool handlers
│   └── Cargo.toml
│
├── vantage-cli/           # Reference Adapter
│   ├── src/
│   │   ├── main.rs        # CLI entry
│   │   └── dispatch.rs    # Intent executor
│   └── Cargo.toml
│
└── vantage-view/          # (Legacy/Viewer)
    └── ...
```

---

## 9. Versioning

| Component | Current | Contract |
|-----------|---------|----------|
| Intent Schema | v1 | Locked, breaking changes = v2 |
| Core API | v0.1.0 | Stable, additive only |
| MCP Tools | v1 | Stable, new tools = new version |
| CLI | v0.1.0 | Follows Core |

---

## 10. Roadmap

### Phase 1: Cognitive Core ✅
- [x] `document_lens`
- [x] `export_to_pdf`
- [x] `check_drift`
- [x] TDD test suite

### Phase 2: Intent Protocol ✅
- [x] Intent Schema v1
- [x] Validation rules
- [x] CLI Reference Adapter

### Phase 3: Thin Clients 🔜
- [ ] Open Code adapter
- [ ] Antigravity adapter
- [ ] VS Code adapter

### Phase 4: Ecosystem
- [ ] Multi-format export (HTML, DOCX)
- [ ] Batch processing
- [ ] CI/CD integration

---

## Appendix A: Intent Examples

### A.1 Lens Intent (CLI)

```json
{
  "intent": "lens",
  "source": "cli",
  "target": {
    "type": "file",
    "path": "README.md"
  },
  "params": {}
}
```

### A.2 Export Intent (VS Code)

```json
{
  "intent": "export",
  "source": "vscode",
  "target": {
    "type": "file",
    "path": "/project/design.md",
    "selection": { "start": 0, "end": 500 }
  },
  "params": {
    "mode": "visual"
  }
}
```

### A.3 Drift Intent (Antigravity)

```json
{
  "intent": "drift",
  "source": "antigravity",
  "target": {
    "type": "workspace",
    "root": "/project"
  },
  "params": {
    "ignore_patterns": ["test_*", "mock_*"]
  }
}
```

---

*This document is the Ground Truth for Vantage architecture. All implementations must conform to this specification.*
