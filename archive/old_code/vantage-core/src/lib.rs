// ============================================================
// INTENT SCHEMA v1 - Cognitive Protocol Layer
// ============================================================
pub mod intent;

use calamine::{Reader, Xlsx, open_workbook};
use docx_rs::*;
use std::fs;
use std::path::Path;
use std::time::SystemTime;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VantageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Calamine error: {0}")]
    Calamine(String),
    #[error("Docx error: {0}")]
    Docx(String),
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("Agent not found: {0}")]
    AgentNotFound(String),
    #[error("Agent busy: {0}")]
    AgentBusy(String),
    #[error("Export error: {0}")]
    ExportError(String),
    #[error("Invalid document: {0}")]
    InvalidDocument(String),
}

#[derive(Debug)]
pub struct Sheet {
    pub name: String,
    pub data: Vec<Vec<String>>,
}

#[derive(Debug)]
pub struct Metadata {
    pub word_count: usize,
    pub last_modified: Option<SystemTime>,
    pub source_type: String,
}

#[derive(Debug)]
pub struct DocumentData {
    pub sheets: Vec<Sheet>,
    pub text: String,
    pub headers: Vec<String>,
    pub metadata: Metadata,
}

pub fn document_lens(path: &str) -> Result<DocumentData, VantageError> {
    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| VantageError::UnsupportedFileType("No extension".to_string()))?;

    let mut sheets = Vec::new();
    let mut text = String::new();
    let mut headers = Vec::new();

    let last_modified = fs::metadata(path).ok().and_then(|m| m.modified().ok());

    match extension {
        "xlsx" => {
            let mut workbook: Xlsx<_> =
                open_workbook(path).map_err(|e| VantageError::Calamine(format!("{:?}", e)))?;
            for sheet_name in workbook.sheet_names() {
                if let Ok(range) = workbook.worksheet_range(&sheet_name) {
                    let mut data = Vec::new();
                    for (i, row) in range.rows().enumerate() {
                        let row_data: Vec<String> =
                            row.iter().map(|cell| cell.to_string()).collect();
                        if i == 0 {
                            headers.extend(row_data.clone());
                        }
                        data.push(row_data);
                    }
                    sheets.push(Sheet {
                        name: sheet_name,
                        data,
                    });
                }
            }
            text = format!("Excel file with {} sheets", sheets.len());
        }
        "docx" => {
            let buf = fs::read(path)?;
            let docx = read_docx(&buf).map_err(|e| VantageError::Docx(e.to_string()))?;
            for paragraph in docx.document.children {
                if let DocumentChild::Paragraph(p) = paragraph {
                    for child in p.children {
                        if let ParagraphChild::Run(r) = child {
                            for text_run in r.children {
                                if let RunChild::Text(t) = text_run {
                                    text.push_str(&t.text);
                                    text.push(' ');
                                }
                            }
                        }
                    }
                    text.push('\n');
                }
            }
        }
        "md" => {
            let content = fs::read_to_string(path).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    VantageError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Document not found at path: {}. Please provide a valid absolute path.", path.display())
                    ))
                } else {
                    VantageError::Io(e)
                }
            })?;
            text = content.clone();
            for line in content.lines() {
                if line.trim().starts_with('#') {
                    let header = line.trim().trim_start_matches('#').trim().to_string();
                    headers.push(header);
                }
            }
        }
        _ => return Err(VantageError::UnsupportedFileType(extension.to_string())),
    }

    let word_count = text.split_whitespace().count();
    let metadata = Metadata {
        word_count,
        last_modified,
        source_type: extension.to_string(),
    };

    Ok(DocumentData {
        sheets,
        text,
        headers,
        metadata,
    })
}

/// Configuration for PDF export
#[derive(Debug, Clone)]
pub struct ExportConfig {
    pub mode: ExportMode,
    pub include_toc: bool,
    pub include_metadata: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            mode: ExportMode::Semantic,
            include_toc: true,
            include_metadata: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportMode {
    Semantic,  // Internal: Preserve structure for agents (Clean)
    Visual,    // External: Optimized for human reading (Clean/Client)
    Audit,     // Internal: Include drift reports, signatures, and timestamps (Signed)
}

/// Export DocumentData to PDF (zero-token primitive)
/// 
/// # Arguments
/// * `doc` - DocumentData from document_lens()
/// * `config` - Export configuration
/// 
/// # Returns
/// * PDF bytes ready to write to file
/// 
/// # Example
/// ```ignore
/// let doc = document_lens("design.md")?;
/// let pdf = export_to_pdf(&doc, ExportConfig::default())?;
/// std::fs::write("output.pdf", pdf)?;
/// ```
pub fn export_to_pdf(
    doc: &DocumentData,
    _config: ExportConfig,
) -> Result<Vec<u8>, VantageError> {
    use printpdf::*;
    
    // Validate input (Safe Rust 2026)
    if doc.text.is_empty() && doc.headers.is_empty() {
        return Err(VantageError::InvalidDocument(
            "Document has no content to export".to_string()
        ));
    }
    
    // Create PDF document
    let (doc_pdf, page1, layer1) = PdfDocument::new(
        "Vantage Export",
        Mm(210.0), // A4 width
        Mm(297.0), // A4 height
        "Layer 1"
    );
    
    let current_layer = doc_pdf.get_page(page1).get_layer(layer1);
    
    // Load font (embedded in printpdf)
    let font = doc_pdf.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| VantageError::ExportError(format!("Font error: {:?}", e)))?;
    
    // Render headers
    let mut y_position = 270.0; // Start near top
    
    for header in &doc.headers {
        current_layer.use_text(
            header,
            14.0,
            Mm(20.0),
            Mm(y_position),
            &font
        );
        y_position -= 10.0;
    }
    
    // Render text content (simplified for MVP)
    y_position -= 10.0;
    let text_lines: Vec<&str> = doc.text.lines().take(20).collect(); // Cap for MVP
    
    for line in text_lines {
        if y_position < 20.0 {
            break; // Avoid page overflow in MVP
        }
        current_layer.use_text(
            line,
            10.0,
            Mm(20.0),
            Mm(y_position),
            &font
        );
        y_position -= 5.0;
    }
    
    // Only sign if in Audit mode
    if _config.mode == ExportMode::Audit {
        let metadata_text = format!("Generated by Vantage | Source: {} | Mode: Audit", doc.metadata.source_type);
        current_layer.use_text(
            &metadata_text,
            8.0,
            Mm(20.0),
            Mm(10.0),
            &font
        );
    }
    
    // Save to bytes
    let buffer = doc_pdf.save_to_bytes()
        .map_err(|e| VantageError::ExportError(format!("PDF save error: {:?}", e)))?;
    
    Ok(buffer)
}


/// Configuration for drift detection
#[derive(Debug, Clone)]
pub struct DriftConfig {
    /// Path to the design/blueprint document (Markdown)
    pub blueprint_path: String,
    /// Path to the source code directory to scan
    pub code_path: String,
    /// Section header in blueprint to search for function mentions (e.g., "## Features")
    pub section_header: Option<String>,
    /// Optional list of patterns to ignore (e.g., ["sqlite3_*", "test_*"])
    pub ignore_patterns: Option<Vec<String>>,
}

/// Result of drift analysis
#[derive(Debug)]
pub struct DriftReport {
    /// Functions found in code but NOT mentioned in blueprint
    pub undocumented: Vec<String>,
    /// Items mentioned in blueprint but NOT found in code
    pub unimplemented: Vec<String>,
    /// Functions that are properly aligned (in both code and docs)
    pub aligned: Vec<String>,
}

/// Analyzes drift between a blueprint document and actual source code.
/// 
/// # Arguments
/// * `config` - Configuration specifying paths and section to check
/// 
/// # Returns
/// * `DriftReport` with undocumented, unimplemented, and aligned items
/// 
/// # Example
/// ```ignore
/// let config = DriftConfig {
///     blueprint_path: "DESIGN.md".to_string(),
///     code_path: "src/".to_string(),
///     section_header: Some("## Public API".to_string()),
/// };
/// let report = check_drift(config)?;
/// ```
pub fn check_drift(config: DriftConfig) -> Result<DriftReport, VantageError> {
    // 1. Read blueprint and extract mentioned items
    let blueprint_content = fs::read_to_string(&config.blueprint_path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            let cwd = std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_default();
            VantageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Blueprint '{}' not found. Current directory: {}", config.blueprint_path, cwd)
            ))
        } else {
            VantageError::Io(e)
        }
    })?;

    let section_header = config.section_header.as_deref().unwrap_or("## ");
    let mut in_section = false;
    let mut blueprint_items: Vec<String> = Vec::new();
    
    for line in blueprint_content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(section_header) || (section_header == "## " && trimmed.starts_with("# ")) {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("##") && !trimmed.starts_with(section_header) {
            break;
        }
        if in_section && !trimmed.is_empty() {
            // Extract function-like names (snake_case words)
            for word in trimmed.split_whitespace() {
                let clean = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if clean.contains('_') && clean.chars().all(|c| c.is_lowercase() || c == '_' || c.is_numeric()) {
                    blueprint_items.push(clean.to_string());
                }
            }
        }
    }
    blueprint_items.sort();
    blueprint_items.dedup();

    // Default patterns + User provided patterns
    let ignore_patterns = config.ignore_patterns.as_deref().unwrap_or(&[]);

    // Filter blueprint items based on patterns (dont expect ignored items to be implemented)
    if !ignore_patterns.is_empty() {
        blueprint_items.retain(|item| {
            !ignore_patterns.iter().any(|pattern| {
                 if pattern.ends_with('*') {
                     item.starts_with(pattern.trim_end_matches('*'))
                 } else {
                     item == pattern
                 }
            })
        });
    }
    let code_path = Path::new(&config.code_path);
    let mut code_functions: Vec<String> = Vec::new();
    
    // Default patterns + User provided patterns
    let ignore_patterns = config.ignore_patterns.as_deref().unwrap_or(&[]);
    
    if code_path.is_file() {
        extract_pub_functions(code_path, &mut code_functions)?;
    } else if code_path.is_dir() {
        scan_directory_for_functions(code_path, &mut code_functions, ignore_patterns)?;
    } else {
        return Err(VantageError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Code path '{}' not found", config.code_path)
        )));
    }
    
    // Filter functions based on patterns
    if !ignore_patterns.is_empty() {
        code_functions.retain(|func| {
            !ignore_patterns.iter().any(|pattern| {
                 // Simple wildcard support: "sqlite3_*" -> starts_with("sqlite3_")
                 if pattern.ends_with('*') {
                     func.starts_with(pattern.trim_end_matches('*'))
                 } else {
                     func == pattern
                 }
            })
        });
    }

    code_functions.sort();
    code_functions.dedup();

    // 3. Compute drift
    let mut undocumented = Vec::new();
    let mut aligned = Vec::new();
    
    for func in &code_functions {
        if blueprint_items.iter().any(|item| item == func) {
            aligned.push(func.clone());
        } else {
            undocumented.push(func.clone());
        }
    }

    let unimplemented: Vec<String> = blueprint_items
        .iter()
        .filter(|item| !code_functions.contains(item))
        .cloned()
        .collect();

    Ok(DriftReport { undocumented, unimplemented, aligned })
}

/// Default directories to ignore during scan
pub const DEFAULT_IGNORE_DIRS: &[&str] = &["target", ".git", "node_modules", "vendor", "dist", "build"];

fn extract_pub_functions(path: &Path, functions: &mut Vec<String>) -> Result<(), VantageError> {
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("pub fn ") || trimmed.starts_with("pub async fn "))
            && let Some(name) = extract_fn_name(trimmed) {
                functions.push(name);
            }
    }
    Ok(())
}

fn extract_fn_name(line: &str) -> Option<String> {
    let start = if line.contains("async fn ") {
        line.find("async fn ")? + 9
    } else {
        line.find("fn ")? + 3
    };
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(rest[..end].trim().to_string())
}

fn scan_directory_for_functions(dir: &Path, functions: &mut Vec<String>, _ignore_patterns: &[String]) -> Result<(), VantageError> {
    // NOTE: _ignore_patterns is intentionally prefixed with underscore as it is 
    // filtered at the check_drift() level after collection for simplicity.
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        
        // Check for ignored directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && DEFAULT_IGNORE_DIRS.contains(&name) {
                continue;
            }

        if path.is_dir() {
            scan_directory_for_functions(&path, functions, _ignore_patterns)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            extract_pub_functions(&path, functions)?;
        }
    }
    
    Ok(())
}

pub fn get_agent_status() -> String {
    "Core System Active: Ready to deploy agents".to_string()
}

/// Generates a PDF snapshot of the current drift status
pub fn export_drift_snapshot(
    config: DriftConfig,
    export_config: ExportConfig,
) -> Result<Vec<u8>, VantageError> {
    // 1. Run the audit
    let report = check_drift(config.clone())?;
    
    // 2. Map report to a DocumentData
    let mut text = format!(
        "VANTAGE AUDIT REPORT\n\nTarget: {}\nBlueprint: {}\n\n",
        config.code_path, config.blueprint_path
    );
    
    text.push_str("--- AUDIT SUMMARY ---\n\n");
    text.push_str(&format!("✅ Aligned: {}\n", report.aligned.len()));
    text.push_str(&format!("⚠️ Undocumented: {}\n", report.undocumented.len()));
    text.push_str(&format!("🚧 Unimplemented: {}\n\n", report.unimplemented.len()));
    
    text.push_str("--- DETAILS ---\n\n");
    
    if !report.undocumented.is_empty() {
        text.push_str("⚠️ UNDOCUMENTED FUNCTIONS (Drift):\n");
        for item in report.undocumented.iter().take(10) {
            text.push_str(&format!("- {}\n", item));
        }
        if report.undocumented.len() > 10 {
            text.push_str("... and more.\n");
        }
        text.push('\n');
    }
    
    if !report.unimplemented.is_empty() {
        text.push_str("🚧 UNIMPLEMENTED SPEC ITEMS:\n");
        for item in report.unimplemented.iter().take(10) {
            text.push_str(&format!("- {}\n", item));
        }
        text.push('\n');
    }
    
    let doc_data = DocumentData {
        sheets: vec![],
        text,
        headers: vec!["AUDIT REPORT".to_string(), "SUMMARY".to_string(), "DETAILS".to_string()],
        metadata: Metadata {
            word_count: 0, // Calculated in to_pdf
            last_modified: Some(SystemTime::now()),
            source_type: "audit_report".to_string(),
        },
    };
    
    // 3. Export to PDF
    export_to_pdf(&doc_data, export_config)
}

// --- PHẦN 1: LOGIC (DOMAIN MODELS) ---

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Developer,
    Tester,
    Manager,
}

#[derive(Debug, Clone)]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub role: AgentRole,
    pub is_busy: bool,
}

pub struct AgentSwarm {
    agents: HashMap<String, Agent>,
}

impl Default for AgentSwarm {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentSwarm {
    // Khởi tạo bầy đàn mới
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    // Spawn (Tạo) một Agent mới
    pub fn spawn_agent(&mut self, name: &str, role: AgentRole) -> String {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let short_id = if uuid_str.len() >= 4 { &uuid_str[0..4] } else { "0000" };
        let id = format!("{}_{}", name, short_id);
        let agent = Agent {
            id: id.clone(),
            name: name.to_string(),
            role,
            is_busy: false,
        };
        self.agents.insert(id.clone(), agent);
        id
    }

    // Kiểm tra quân số
    pub fn get_agent_count(&self) -> usize {
        self.agents.len()
    }

    // Giao việc (Đơn giản hóa)
    pub fn assign_task(&mut self, agent_id: &str) -> Result<String, VantageError> {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            if agent.is_busy {
                Err(VantageError::AgentBusy(agent.name.clone()))
            } else {
                agent.is_busy = true;
                Ok(format!("Đã giao việc cho {}", agent.name))
            }
        } else {
            Err(VantageError::AgentNotFound(agent_id.to_string()))
        }
    }
}

// --- PHẦN 2: TDD UNIT TESTS (Cái bác đang cần) ---

#[cfg(test)]
mod tests {
    use super::*;

    // Test 1: Khởi động hệ thống
    #[test]
    fn test_swarm_initialization() {
        let swarm = AgentSwarm::new();
        assert_eq!(swarm.get_agent_count(), 0, "Bầy đàn ban đầu phải rỗng");
    }

    // Test 2: Spawn Agent xem có ra không
    #[test]
    fn test_spawn_agent() {
        let mut swarm = AgentSwarm::new();
        let id = swarm.spawn_agent("Dev_Zero", AgentRole::Developer);

        assert_eq!(swarm.get_agent_count(), 1);
        assert!(id.contains("Dev_Zero"), "ID phải chứa tên agent");
    }

    // Test 3: Giao việc (Task Assignment Flow)
    #[test]
    fn test_task_assignment() {
        let mut swarm = AgentSwarm::new();
        let id = swarm.spawn_agent("Tester_One", AgentRole::Tester);

        // Giao việc lần 1 -> Phải OK
        let result = swarm.assign_task(&id);
        assert!(result.is_ok());

        // Giao việc lần 2 -> Phải Báo bận (Fail)
        let result_busy = swarm.assign_task(&id);
        assert!(result_busy.is_err());
        match result_busy {
            Err(VantageError::AgentBusy(name)) => assert_eq!(name, "Tester_One"),
            _ => panic!("Expected AgentBusy error"),
        }
    }

    // ============================================================
    // TDD TESTS FOR DOCUMENT_LENS
    // ============================================================

    #[test]
    fn test_document_lens_unsupported_extension() {
        let result = document_lens("test.xyz");
        assert!(result.is_err());
        match result {
            Err(VantageError::UnsupportedFileType(ext)) => assert_eq!(ext, "xyz"),
            _ => panic!("Expected UnsupportedFileType error"),
        }
    }

    #[test]
    fn test_document_lens_missing_file() {
        let result = document_lens("nonexistent.md");
        assert!(result.is_err());
        // Should be IO error, not unsupported type
        assert!(matches!(result, Err(VantageError::Io(_))));
    }

    // ============================================================
    // TDD TESTS FOR CHECK_DRIFT (Generalized)
    // ============================================================

    #[test]
    fn test_check_drift_missing_blueprint() {
        let config = DriftConfig {
            blueprint_path: "DOES_NOT_EXIST.md".to_string(),
            code_path: "src/".to_string(),
            section_header: None,
            ignore_patterns: None,
        };
        let result = check_drift(config);
        assert!(result.is_err());
        assert!(matches!(result, Err(VantageError::Io(_))));
    }

    #[test]
    fn test_check_drift_missing_code_path() {
        // Create a temp blueprint file for this test
        let temp_dir = std::env::temp_dir();
        let blueprint = temp_dir.join("test_blueprint.md");
        std::fs::write(&blueprint, "## Features\n- some_feature\n").expect("Test setup: failed to write blueprint");
        
        let config = DriftConfig {
            blueprint_path: blueprint.to_string_lossy().to_string(),
            code_path: "NONEXISTENT_DIR/".to_string(),
            section_header: Some("## Features".to_string()),
            ignore_patterns: None,
        };
        let result = check_drift(config);
        assert!(result.is_err());
        
        // Cleanup
        let _ = std::fs::remove_file(blueprint);
    }

    #[test]
    fn test_extract_fn_name() {
        assert_eq!(extract_fn_name("pub fn hello_world()"), Some("hello_world".to_string()));
        assert_eq!(extract_fn_name("pub async fn fetch_data()"), Some("fetch_data".to_string()));
        assert_eq!(extract_fn_name("fn private_func()"), Some("private_func".to_string()));
        assert_eq!(extract_fn_name("let x = 5;"), None);
    }

    #[test]
    fn test_drift_report_structure() {
        // This test validates the DriftReport struct has correct fields
        let report = DriftReport {
            undocumented: vec!["func_a".to_string()],
            unimplemented: vec!["func_b".to_string()],
            aligned: vec!["func_c".to_string()],
        };
        assert_eq!(report.undocumented.len(), 1);
        assert_eq!(report.unimplemented.len(), 1);
        assert_eq!(report.aligned.len(), 1);
    }
}
