---
name: Rust MCP Server Development (Elite 9)
description: A comprehensive guide to building, refactoring, and maintaining Model Context Protocol (MCP) servers in Rust, emphasizing TDD-First and Safe Rust practices.
version: 1.0.0
---

# Rust MCP Server Development (Elite 9 Standard)

This skill encapsulates the **"Sustainable Dev Cycle"** for building robust, future-proof MCP extensions. It is forged from the experience of refactoring `vantage-core` and integrating `rust-mcp-sdk`.

## I. Core Philosophy: The TDD-First "Long Mach"

**"Write Test as Specification"**

1.  **TDD First (Behavior Design)**:
    *   Do not start with `main.rs`. Start with `tests/`.
    *   Define behavior via tests: "If I call `spawn_agent`, what exactly happens?"
    *   Use Rust's **Type System** to design the API before implementation (Type-Driven Development).
    *   *Rule:* No implementation code exists without a failing test (Red phase).

2.  **Architecture (Hexagonal/Ports & Adapters)**:
    *   **Core (Library)**: Pure business logic. Zero dependencies on external transport/servers if possible.
        *   Testable in isolation via Unit Tests.
        *   Uses `thiserror` for library-level errors.
    *   **Adapter (MCP Server)**: The glue code (e.g., `main.rs`).
        *   Maps Core Logic to MCP Tools.
        *   Uses `anyhow` or `SdkResult` for application-level errors.
        *   Decoupled from Core via Traits.

3.  **Safety Net (Refactoring Confidence)**:
    *   Never refactor Legacy Code without Characterization Tests.
    *   **CI/CD Rule**: `cargo test` must pass before `cargo build` or merge.
    *   Tests allow you to survive SDK breaking changes (like 0.7.x -> 0.8.x).

## 🚨 GOLDEN RULE: Absolute Paths Only

MCP servers run in their **own working directory**, NOT the user's workspace.
- ❌ `"path": "src/lib.rs"` → Will fail in other IDEs
- ✅ `"path": "E:\\\\MyProject\\\\src\\\\lib.rs"` → Works everywhere

**All file-based tools MUST:**
1. Require absolute paths in parameters
2. State "⚠️ REQUIRES ABSOLUTE PATH" in tool description
3. Show full path examples: `'E:\\\\Projects\\\\file.md'` or `'/home/user/file.md'`

## 🚀 Zero-Config Auto-Detection Pattern

**The best UX = No user input required.** MCP protocol provides `request_root_list()` to get workspace from IDE.

```rust
async fn auto_detect_workspace(runtime: &Arc<dyn McpServer>, context: ...) -> Option<PathBuf> {
    // Check if IDE supports roots
    if runtime.client_supports_root_list() != Some(true) {
        return None;
    }
    
    // Request roots from IDE - ZERO user input!
    match runtime.request_root_list(None).await {
        Ok(roots_result) => {
            if let Some(first_root) = roots_result.roots.first() {
                // Parse file:/// URI to PathBuf
                let path = PathBuf::from(first_root.uri.trim_start_matches("file:///"));
                ctx.workspace_root = Some(path.clone());
                return Some(path);
            }
        }
        Err(_) => {} // Fallback to manual set_workspace
    }
    None
}
```

**UX Flow (IDE-first):**
1. User calls `check_drift(blueprint: "README.md", code: "src/")`
2. VANTAGE auto-detects workspace from IDE via MCP roots
3. Paths resolved automatically - **user never types absolute paths!**
4. If IDE doesn't support roots, falls back to `set_workspace`

## II. Safe Rust 2026 Standards

Strict adherence to correct error handling ensures the server never crashes (panics).

1.  **Zero Panic Policy**:
    *   ❌ No `unsafe` blocks.
    *   ❌ No `.unwrap()` or `.expect()` (except in early prototype tests).
    *   ✅ Use `?` operator for error propagation.

2.  **Error Handling Pipeline**:
    ```rust
    // 1. Library Error (vantage-core/src/lib.rs)
    #[derive(Error, Debug)]
    pub enum CoreError {
        #[error("IO failed: {0}")]
        Io(#[from] std::io::Error),
    }

    // 2. Boundary Conversion (vantage-mcp/src/main.rs)
    match core_function() {
        Ok(val) => val,
        Err(e) => return Err(CallToolError::from_message(e.to_string())), // Map to MCP error
    }
    ```

## III. Implementation Patterns (rust-mcp-sdk)

### 1. Handling SDK Version Mismatches
Always check version compatibility. The SDK moves fast.
```bash
cargo tree -p rust-mcp-sdk
```
*Lesson*: If `lib` and `bin` diverge in SDK versions, compilation hell ensures. Align versions immediately (Upgrade preferred over Downgrade).

### 2. Tool Definition (JSON Pattern)
Avoid using rigid Structs for `Tool` definition if SDK versions fluctuate. Use `serde_json` for flexibility:

```rust
// ✅ Resilient Pattern
let tool = serde_json::from_value(json!({
    "name": "my_tool",
    "description": "...",
    "inputSchema": {
        "type": "object",
        "properties": { ... },
        "required": [...]
    }
})).map_err(|e| RpcError::internal_error())?;
```

### 3. Server Handler Trait (SDK 0.8.x+)
Implement the `ServerHandler` trait for your handler struct.

```rust
#[async_trait]
impl ServerHandler for MyHandler {
    async fn handle_list_tools_request(...) -> Result<ListToolsResult, RpcError> { ... }
    
    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        ...
    ) -> Result<CallToolResult, CallToolError> {
        // Match tool name and execute core logic
    }
}
```

### 4. Server Initialization
Use the `server_runtime` factory:

```rust
let server = server_runtime::create_server(McpServerOptions {
    server_details,
    transport,
    handler: my_handler.to_mcp_server_handler(),
    ...
});
if let Err(e) = server.start().await { ... }
```

## IV. Generalized Tool Design (Lesson from check_drift)

**Anti-pattern (WRONG):**
```rust
// ❌ Hardcoded self-referential logic
let functions = vec!["document_lens", "check_drift"]; // Checking itself!
```

**Correct Pattern:**
```rust
// ✅ Configurable, works for ANY project
pub struct DriftConfig {
    pub blueprint_path: String,  // User provides
    pub code_path: String,       // User provides  
    pub section_header: Option<String>,
}

pub fn check_drift(config: DriftConfig) -> Result<DriftReport, Error> {
    // Scans ACTUAL code files, not hardcoded list
}
```

## V. TDD Test Examples

```rust
#[test]
fn test_check_drift_missing_blueprint() {
    let config = DriftConfig {
        blueprint_path: "DOES_NOT_EXIST.md".to_string(),
        code_path: "src/".to_string(),
        section_header: None,
    };
    let result = check_drift(config);
    assert!(result.is_err());
    assert!(matches!(result, Err(VantageError::Io(_))));
}

#[test]
fn test_extract_fn_name() {
    assert_eq!(extract_fn_name("pub fn hello_world()"), Some("hello_world".to_string()));
    assert_eq!(extract_fn_name("pub async fn fetch_data()"), Some("fetch_data".to_string()));
}
```

## VI. Workflow for New Extensions

1.  **Define Core Logic** (`vantage-core`):
    *   Write test: `test_my_feature()`.
    *   Implement feature in `lib.rs`.
    *   Verify: `cargo test -p vantage-core`.

2.  **Expose via MCP** (`vantage-mcp`):
    *   Update `handle_list_tools_request`: Register new tool JSON.
    *   Update `handle_call_tool_request`: Add match arm calling core logic.
    *   Manual Verify: `cargo check`.

3.  **Final Polish**:
    *   Run `cargo clippy`.
    *   Update documentation.

## VII. The "Identity Split" Pattern (Audit vs Delivery)

When generating artifacts (PDF, Excel) for both internal and external use, do NOT use "hide flag" logic. Use **Identity Segregation**.

1.  **Separation of Concerns**: Define `ExportMode` as a first-class citizen in the domain model.
2.  **Audit Mode**: Explicitly includes the "Cognitive Signature" (Provenance, Timestamp, Hash).
3.  **Visual Mode**: Explicitly excludes all non-essential metadata for professional client delivery.
4.  **Semantic Mode**: Optimized for Agent consumption (structured but clean).

*Rule*: Professionalism is triacted via **Architectural Compliance**, not UI checkboxes.

## VIII. Testing Deterministic Binary Artifacts

Rust libraries like `printpdf` often embed non-deterministic metadata (e.g., `CreationDate`).

1.  **Primary Test**: High-integrity byte-to-byte comparison (locks your code).
2.  **The Metadata Trap**: If external libraries inject timestamps, byte-to-byte will fail.
3.  **Fallback Strategy**:
    *   **Length Stability**: `assert_eq!(pdf1.len(), pdf2.len())`. While not perfect, it flags unexpected content changes.
    *   **Content Markers**: Verify known magic numbers (e.g., `%PDF`) and footer markers.
    *   **Technical Debt**: Explicitly document why a test is not byte-perfect using `TODO: Use lopdf to scrub timestamps`.

---
*Updated for Vantage Phase 1 - 2026-01-20*
