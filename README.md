# VANTAGE: Spatial AI Orchestration Protocol

VANTAGE is a Rust-based ecosystem for managing, auditing, and visualizing AI Agent Swarms. It provides a spatial "Head-Up Display" (HUD) for developers to orchestrate complex AI tasks without losing context.

## 🏗 Project Structure

This project is organized as a Rust Workspace:

*   **`vantage-core`** (✅ Stable): The brain. Contains `AgentSwarm` logic, task delegation, and drift detection.
*   **`vantage-mcp`** (✅ Universal): The bridge connecting Vantage to any IDE via Model Context Protocol.
*   **`vantage-view`** (✅ Tested): The frontend renderer for the Spatial HUD.

## 🚀 Setup & Usage (Antigravity / Claude Code)

1. **Build the Binary**:
   ```bash
   cargo build --release -p vantage-mcp
   ```
2. **Connect to IDE**:
   - Path: `E:\RESEARCH\Vantage\target\release\vantage-mcp.exe`
   - Config: See `vantage-mcp.json` for manual settings.

3. **Zero-Config Workflow**:
   VANTAGE automatically identifies your project root. Just call tools with relative paths!

## 🛠 Core Tools
*   `check_core_status`: Diagnostic & workspace context.
*   `spawn_agent`: Recruit specialized AI agents.
*   `document_lens`: Deep-scan Excel, Word, and Markdown.
*   `check_drift`: Audit code against your design blueprint.

## 🛡 Safe Rust 2026
Built with **TDD-First** methodology and strictly adhering to **Safe Rust** standards:
- Zero `unsafe` blocks
- No `.unwrap()` or `.expect()` in production code
- Comprehensive error handling via `thiserror`

---
*VANTAGE - Orchestrating Intelligence in Space.*