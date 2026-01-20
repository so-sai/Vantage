//! VANTAGE MCP Server - Universal Plugin
//! This server exposes vantage-core functionalities as MCP Tools.
//! 
//! ## Auto-Detection (Zero Configuration)
//! VANTAGE automatically detects your workspace from IDE:
//! 1. On first tool call: Auto-fetches roots from IDE via MCP protocol
//! 2. Falls back to explicit `set_workspace` if IDE doesn't support roots
//! 3. All subsequent calls use resolved workspace - no manual paths needed!

use async_trait::async_trait;
use rust_mcp_sdk::mcp_server::{server_runtime, McpServerOptions, ServerHandler};
use rust_mcp_sdk::schema::{
    schema_utils::CallToolError, CallToolRequestParams, CallToolResult, Implementation,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion, RpcError,
    ServerCapabilities, ServerCapabilitiesTools, TextContent, Tool,
};
use rust_mcp_sdk::{McpServer, StdioTransport, ToMcpServerHandler, TransportOptions};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use vantage_core::{AgentRole, AgentSwarm};

/// Session context for auto-detection and progressive automation
#[derive(Debug, Default)]
struct SessionContext {
    /// Workspace root (auto-detected or manually set)
    workspace_root: Option<PathBuf>,
    /// Whether auto-detection has been attempted
    auto_detected: bool,
}

impl SessionContext {
    /// Resolve a path - if relative, prepend workspace root
    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = PathBuf::from(path);
        if p.is_absolute() {
            p
        } else if let Some(ref root) = self.workspace_root {
            root.join(path)
        } else {
            p
        }
    }

    /// Set workspace root explicitly
    fn set_workspace(&mut self, path: PathBuf) {
        self.workspace_root = Some(path);
        self.auto_detected = true;
    }
}

/// Auto-detect workspace roots from IDE via MCP protocol
async fn auto_detect_workspace(runtime: &Arc<dyn McpServer>, context: &Arc<Mutex<SessionContext>>) -> Option<PathBuf> {
    let mut ctx = context.lock().await;
    
    // Already have workspace or already tried
    if ctx.workspace_root.is_some() || ctx.auto_detected {
        return ctx.workspace_root.clone();
    }
    
    ctx.auto_detected = true;
    
    // Check if IDE supports roots
    if runtime.client_supports_root_list() != Some(true) {
        return None;
    }
    
    // Request roots from IDE
    match runtime.request_root_list(None).await {
        Ok(roots_result) => {
            if let Some(first_root) = roots_result.roots.first() {
                // Parse URI to path (file:///path/to/workspace -> /path/to/workspace)
                let uri = &first_root.uri;
                let path = if uri.starts_with("file:///") {
                    // Windows: file:///C:/path -> C:/path
                    PathBuf::from(uri.trim_start_matches("file:///"))
                } else if uri.starts_with("file://") {
                    // Unix: file:///path -> /path
                    PathBuf::from(uri.trim_start_matches("file://"))
                } else {
                    PathBuf::from(uri)
                };
                ctx.workspace_root = Some(path.clone());
                return Some(path);
            }
        }
        Err(_) => {
            // IDE doesn't support roots, silently continue
        }
    }
    None
}

/// The main handler for VANTAGE MCP Server
struct VantageHandler {
    swarm: Arc<Mutex<AgentSwarm>>,
    context: Arc<Mutex<SessionContext>>,
}

impl VantageHandler {
    fn new() -> Self {
        Self {
            swarm: Arc::new(Mutex::new(AgentSwarm::new())),
            context: Arc::new(Mutex::new(SessionContext::default())),
        }
    }
}

#[async_trait]
impl ServerHandler for VantageHandler {
    async fn handle_list_tools_request(
        &self,
        _params: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        // Create tools using JSON and serde_json to avoid struct field mismatches
        let tools: Vec<Tool> = vec![
            serde_json::from_value(json!({
                "name": "set_workspace",
                "description": "🏠 Set your project workspace root. After this, all other tools can use relative paths. Call this FIRST for best experience.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "ABSOLUTE path to your project root. Example: 'E:\\\\MyProject' or '/home/user/project'"}
                    },
                    "required": ["path"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "check_core_status",
                "description": "💎 Crystal-clear diagnostic of the VANTAGE core engine. Returns current operational readiness and workspace context.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "spawn_agent",
                "description": "🚀 Orchestrate a new autonomous agent within the VANTAGE swarm. Personalize by name and specialized role.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string", "description": "The unique callsign for your new agent. Example: 'DataNavigator'"},
                        "role": {"type": "string", "enum": ["Developer", "Tester", "Manager"], "description": "The specialized cognitive profile for the agent."}
                    },
                    "required": ["name", "role"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "document_lens",
                "description": "🔍 Spatial analysis lens for documents (Excel, Word, Markdown). 💡 TIP: Use set_workspace first, then relative paths work!",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path to document. If workspace is set, can be relative. Otherwise use absolute path."}
                    },
                    "required": ["path"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "check_drift",
                "description": "📡 Universal Code-vs-Docs Drift Detector. 💡 TIP: Use set_workspace first, then relative paths work!",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "blueprint_path": {"type": "string", "description": "Path to design document. Can be relative if workspace is set."},
                        "code_path": {"type": "string", "description": "Path to source directory or file. Can be relative if workspace is set."},
                        "section": {"type": "string", "description": "Optional: Section header in blueprint to focus on. Example: '## Public API'"}
                    },
                    "required": ["blueprint_path", "code_path"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "export_pdf",
                "description": "📄 Export document to PDF (zero-token primitive). Preserves semantic structure for agents. 💡 TIP: Use set_workspace first!",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "file_path": {"type": "string", "description": "Path to document (.md, .docx, .xlsx). Can be relative if workspace is set."},
                        "output_path": {"type": "string", "description": "Optional: Output PDF path. Defaults to <file_path>.pdf"},
                        "mode": {"type": "string", "description": "Export mode: 'visual' (clean for clients), 'audit' (signed for internal), 'semantic' (default)", "enum": ["visual", "audit", "semantic"]}
                    },
                    "required": ["file_path"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
            serde_json::from_value(json!({
                "name": "export_drift_report",
                "description": "📡 Generate a signed PDF Audit Report comparing Spec vs Code. 💡 TIP: Use set_workspace first!",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "blueprint_path": {"type": "string", "description": "Path to design document."},
                        "code_path": {"type": "string", "description": "Path to source directory or file."},
                        "output_path": {"type": "string", "description": "Optional: Output PDF path. Defaults to 'audit_report.pdf'"},
                        "mode": {"type": "string", "description": "Report mode: 'audit' (signed, default), 'visual' (clean)", "enum": ["audit", "visual"]}
                    },
                    "required": ["blueprint_path", "code_path"]
                }
            })).map_err(|e| RpcError::internal_error().with_message(e.to_string()))?,
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        let tool_name = &params.name;
        let args = match params.arguments {
            Some(a) => a,
            None => serde_json::Map::new(),
        };

        let result_text = match tool_name.as_str() {
            "set_workspace" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ Workspace path is required. Example: 'E:\\\\MyProject'"))?;
                
                let abs_path = PathBuf::from(path);
                if !abs_path.is_absolute() {
                    return Err(CallToolError::from_message("⚠️ Workspace path MUST be absolute. Example: 'E:\\\\MyProject' or '/home/user/project'"));
                }
                if !abs_path.exists() {
                    return Err(CallToolError::from_message(format!("⚠️ Path does not exist: {}", path)));
                }

                let mut ctx = self.context.lock().await;
                ctx.set_workspace(abs_path.clone());
                format!("🏠 **Workspace Set!**\n\n📁 Root: `{}`\n\n✨ Now you can use relative paths in all other tools!", abs_path.display())
            }
            "check_core_status" => {
                // Try auto-detection first
                let detected = auto_detect_workspace(&_runtime, &self.context).await;
                
                let ctx = self.context.lock().await;
                let workspace_info = match (&ctx.workspace_root, detected.is_some()) {
                    (Some(p), true) => format!("📁 Workspace: `{}` *(auto-detected from IDE)*", p.display()),
                    (Some(p), false) => format!("📁 Workspace: `{}` *(manually set)*", p.display()),
                    (None, _) => "📁 Workspace: *Not detected* - Use `set_workspace` or ensure IDE provides roots".to_string(),
                };
                format!("✨ {}\n\n{}", vantage_core::get_agent_status(), workspace_info)
            }
            "spawn_agent" => {
                let name = args
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ Agent name is required. Example: 'Agent_007'"))?;
                let role_str = args
                    .get("role")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ Agent role is required. Choose from Developer, Tester, or Manager."))?;
                
                let role = match role_str {
                    "Tester" => AgentRole::Tester,
                    "Manager" => AgentRole::Manager,
                    _ => AgentRole::Developer,
                };
                
                let mut guard = self.swarm.lock().await;
                let id = guard.spawn_agent(name, role);
                format!("✅ Agent Orchestration Successful! ID: **{}**", id)
            }
            "document_lens" => {
                // Auto-detect workspace from IDE
                auto_detect_workspace(&_runtime, &self.context).await;
                
                let path_str = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ Document path is required."))?;
                
                // Resolve path through context (supports relative if workspace is set)
                let resolved_path = {
                    let ctx = self.context.lock().await;
                    ctx.resolve_path(path_str)
                };
                let path_display = resolved_path.display().to_string();
                
                match vantage_core::document_lens(&path_display) {
                    Ok(data) => format!(
                        "📊 **VANTAGE Analysis Report**\n\n📌 **Source**: {}\n📂 **Type**: {}\n📝 **Word Count**: {}\n🔖 **Headers**: {:?}\n\n*Lens analysis complete.*",
                        path_display, data.metadata.source_type, data.metadata.word_count, data.headers
                    ),
                    Err(e) => return Err(CallToolError::from_message(format!("❌ Lens Error: {}", e))),
                }
            }
            "export_pdf" => {
                // Auto-detect workspace from IDE
                auto_detect_workspace(&_runtime, &self.context).await;
                
                let file_path_str = args
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ file_path is required."))?;
                
                let output_path_str = args.get("output_path").and_then(|v| v.as_str());
                
                // Resolve paths through context
                let (file_path, output_path) = {
                    let ctx = self.context.lock().await;
                    let resolved_file = ctx.resolve_path(file_path_str).display().to_string();
                    let resolved_output = if let Some(out) = output_path_str {
                        ctx.resolve_path(out).display().to_string()
                    } else {
                        format!("{}.pdf", resolved_file.trim_end_matches(|c| c != '.'))
                    };
                    (resolved_file, resolved_output)
                };
                
                // Parse document
                let doc = match vantage_core::document_lens(&file_path) {
                    Ok(d) => d,
                    Err(e) => return Err(CallToolError::from_message(format!("❌ Failed to parse document: {}", e))),
                };
                
                let mode_str = args.get("mode").and_then(|v| v.as_str()).unwrap_or("semantic");
                let export_mode = match mode_str {
                    "visual" => vantage_core::ExportMode::Visual,
                    "audit" => vantage_core::ExportMode::Audit,
                    _ => vantage_core::ExportMode::Semantic,
                };
                
                // Export to PDF
                let pdf_bytes = match vantage_core::export_to_pdf(&doc, vantage_core::ExportConfig { mode: export_mode, ..Default::default() }) {
                    Ok(bytes) => bytes,
                    Err(e) => return Err(CallToolError::from_message(format!("❌ PDF export failed: {}", e))),
                };
                
                // Write to file
                if let Err(e) = std::fs::write(&output_path, &pdf_bytes) {
                    return Err(CallToolError::from_message(format!("❌ Failed to write PDF: {}", e)));
                }
                
                format!("✅ **PDF Export Complete!**\n\n📄 Source: `{}`\n📁 Output: `{}`\n📊 Size: {} bytes\n\n*Zero-token primitive executed successfully.*", 
                    file_path, output_path, pdf_bytes.len())
            }
            "check_drift" => {
                // Auto-detect workspace from IDE
                auto_detect_workspace(&_runtime, &self.context).await;
                
                let blueprint_str = args
                    .get("blueprint_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ blueprint_path is required."))?;
                let code_str = args
                    .get("code_path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| CallToolError::from_message("⚠️ code_path is required."))?;
                let section = args.get("section").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                // Parse optional ignore_patterns from args (if user wants to add more)
                let user_ignores: Vec<String> = args.get("ignore_patterns")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                // Advanced Default Patterns (Smart Filtering)
                // User requested FULL TRANSPARENCY - so no defaults like "sqlite3_*" or "test_*"
                // We want to see the trash so we can clean it!
                let ignore_patterns = user_ignores;

                // Resolve paths through context
                let (blueprint_path, code_path) = {
                    let ctx = self.context.lock().await;
                    (ctx.resolve_path(blueprint_str).display().to_string(),
                     ctx.resolve_path(code_str).display().to_string())
                };

                let config = vantage_core::DriftConfig {
                    blueprint_path,
                    code_path,
                    section_header: section,
                    ignore_patterns: Some(ignore_patterns),
                };

                match vantage_core::check_drift(config) {
                    Ok(report) => {
                        let mut output = String::new();
                        output.push_str("📡 **VANTAGE Drift Report 3.0**\n\n");
                        output.push_str("📋 **Target**: Project Blueprint (Abstracted)\n\n");
                        
                        if !report.aligned.is_empty() {
                            output.push_str(&format!("✅ **Aligned** ({}): {}\n\n", report.aligned.len(), report.aligned.join(", ")));
                        }
                        
                        // Smart Display for Undocumented items
                        let undoc_count = report.undocumented.len();
                        if undoc_count > 0 {
                            let display_cap = 20;
                            output.push_str(&format!("⚠️ **Undocumented in blueprint** ({} found):\n", undoc_count));
                            
                            // Show top N items
                            for item in report.undocumented.iter().take(display_cap) {
                                output.push_str(&format!("- `{}`\n", item));
                            }
                            
                            // Show remaining count
                            if undoc_count > display_cap {
                                output.push_str(&format!("- *...and {} more (hidden to reduce noise)*\n", undoc_count - display_cap));
                                output.push_str("> 💡 *Tip: Use `ignore_patterns` to filter specific prefixes like 'utils_*'.*\n");
                            }
                            output.push_str("\n");
                        }

                        if !report.unimplemented.is_empty() {
                            output.push_str(&format!("🚧 **Mentioned but not implemented** ({}):\n- {}\n\n", report.unimplemented.len(), report.unimplemented.join("\n- ")));
                        }
                        if report.undocumented.is_empty() && report.unimplemented.is_empty() {
                            output.push_str("🌈 **Perfect Alignment!** Code and docs are in sync.");
                        }
                        output
                    }
                    Err(e) => return Err(CallToolError::from_message(format!("❌ Drift Audit Failed: {}", e))),
                }
            }
            "export_drift_report" => {
                auto_detect_workspace(&_runtime, &self.context).await;
                
                let blueprint_str = args.get("blueprint_path").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::from_message("⚠️ blueprint_path is required."))?;
                let code_str = args.get("code_path").and_then(|v| v.as_str()).ok_or_else(|| CallToolError::from_message("⚠️ code_path is required."))?;
                let output_path_str = args.get("output_path").and_then(|v| v.as_str()).unwrap_or("audit_report.pdf");
                
                let (blueprint_path, code_path, output_path) = {
                    let ctx = self.context.lock().await;
                    (ctx.resolve_path(blueprint_str).display().to_string(),
                     ctx.resolve_path(code_str).display().to_string(),
                     ctx.resolve_path(output_path_str).display().to_string())
                };
                
                let config = vantage_core::DriftConfig {
                    blueprint_path,
                    code_path,
                    section_header: None,
                    ignore_patterns: None,
                };
                
                let mode_str = args.get("mode").and_then(|v| v.as_str()).unwrap_or("audit");
                let export_mode = match mode_str {
                    "visual" => vantage_core::ExportMode::Visual,
                    _ => vantage_core::ExportMode::Audit,
                };

                let mut export_config = vantage_core::ExportConfig::default();
                export_config.mode = export_mode;
                
                match vantage_core::export_drift_snapshot(config, export_config) {
                    Ok(bytes) => {
                        if let Err(e) = std::fs::write(&output_path, &bytes) {
                            return Err(CallToolError::from_message(format!("❌ Failed to write report: {}", e)));
                        }
                        format!("✅ **Audit Report Generated!**\n\n📄 Target: `{}`\n📁 Saved to: `{}`\n\n*Signed by Vantage - Zero-token Audit.*", code_str, output_path)
                    }
                    Err(e) => return Err(CallToolError::from_message(format!("❌ Audit Failed: {}", e))),
                }
            }
            _ => return Err(CallToolError::unknown_tool(tool_name)),
        };

        Ok(CallToolResult::text_content(vec![TextContent::from(
            result_text,
        )]))
    }
}

#[tokio::main]
async fn main() -> rust_mcp_sdk::error::SdkResult<()> {
    eprintln!("Vantage Universal MCP Server starting...");

    // Define server details and capabilities
    let server_details = InitializeResult {
        server_info: Implementation {
            name: "vantage-mcp".to_string(),
            version: "0.1.0".to_string(),
            title: Some("VANTAGE Universal Plugin".to_string()),
            description: Some(
                "Universal extraction and agent orchestration for Antigravity".to_string(),
            ),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        meta: None,
        instructions: Some(
            "Use vantage tools to analyze documents, spawn agents, and check system drift."
                .to_string(),
        ),
        protocol_version: ProtocolVersion::V2025_11_25.into(),
    };

    // Create transport
    let transport = StdioTransport::new(TransportOptions::default())?;

    // Instantiate handler
    let handler = VantageHandler::new();

    // Create MCP server
    let server = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    eprintln!("Vantage Server running on stdio...");
    
    // Start the server with proper error handling
    if let Err(start_error) = server.start().await {
        eprintln!(
            "{}",
            start_error
                .rpc_error_message()
                .unwrap_or(&start_error.to_string())
        );
    }

    Ok(())
}
