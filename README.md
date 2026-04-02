# Vantage v1.2.4

Structural code sensor with decision engine and symbol graph. Detects drift, classifies claims, enforces invariants, maps dependencies — single-shot CLI.

## Install

```bash
cargo build -p vantage-core -p vantage-cli --release
```

## Quick Start

```bash
# Verify a file
vantage verify src/main.rs --json

# Verify with enforcement (signal → claim → invariant → verdict)
vantage verify src/main.rs --enforce

# Show dependency graph + impact radius
vantage graph src/main.rs

# Establish baseline
vantage seal .

# Compare against baseline
vantage diff src/main.rs
```

## CLI Reference

| Command | Description |
|---------|-------------|
| `vantage verify <file>` | Analyze file, output structural signals |
| `vantage verify <file> --json` | JSON output |
| `vantage verify <file> --enforce` | Run decision pipeline (Allow/Warn/Reject) |
| `vantage graph <file>` | Show dependency graph + impact radius |
| `vantage graph <file> --json` | JSON graph output |
| `vantage diff <file>` | Compare against VANTAGE.SEAL baseline |
| `vantage seal <dir>` | Create forensic baseline |
| `vantage purge --force` | Remove local artifacts |

## Key Concepts

**Triple Hash Identity**:
- `structural_hash` — byte-level change detector
- `semantic_hash` — whitespace-invariant
- `normalized_hash` — rename-invariant (strips identifiers from AST)

**Epistemic Anchors**: Mark code blocks with `# @epistemic:<uuid>` in comments.

**Symbol Graph**: Zero-semantic dependency graph. Shows `A → calls → B` relationships. Use `impact_radius()` to find which callers are affected by a change.

**Drift Report**: Function-level diff showing which symbols changed, added, or removed.

**Execution Pipeline**: `signal → claim → invariant → verdict` with `--enforce` flag.

## For Agents

Read `AGENTS.md` for the agent contract. Read `SPECS.md` for technical details.

## Integration

Vantage is a plugin for the [kit](https://github.com/anomalyco/kit) ecosystem.

---
*v1.2.4 — Knowledge Router Operational*
