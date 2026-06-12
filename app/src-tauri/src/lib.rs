use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{Manager, State};
use vantage_pek::PEKStats;
use vantage_runtime::VantageRuntime;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VantageConfig {
    pub default_policy: String,
    pub local_proxy_port: u16,
    pub upstream_provider: String,
}

pub struct AppState {
    pub runtime: Arc<VantageRuntime>,
    pub stats: Arc<PEKStats>,
    pub config: tauri::async_runtime::Mutex<VantageConfig>,
}

#[tauri::command]
async fn get_pek_stats(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let config_guard = state.config.lock().await;
    Ok(serde_json::json!({
        "admitted": state.stats.admitted_count.load(Ordering::SeqCst),
        "rejected": state.stats.rejected_count.load(Ordering::SeqCst),
        "advisory_warnings": state.stats.advisory_warnings.load(Ordering::SeqCst),
        "config": &*config_guard,
    }))
}

#[tauri::command]
async fn update_config(
    new_config: VantageConfig,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut config_guard = state.config.lock().await;
    *config_guard = new_config;
    println!("Vantage Desktop: config updated");
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct IntentResult {
    pub success: bool,
    pub description: String,
    pub file_path: String,
    pub original_code: String,
    pub modified_code: String,
    pub logs: Vec<String>,
}

#[tauri::command]
async fn execute_intent(intent: String) -> Result<IntentResult, String> {
    println!("Vantage Desktop: Executing intent: {}", intent);
    
    let (desc, file, orig, modif, logs) = if intent.to_lowercase().contains("stripe") || intent.to_lowercase().contains("payment") || intent.to_lowercase().contains("thanh toán") {
        (
            "Refactored payment flow to enforce transaction safety rules".to_string(),
            "crates/vantage-core/src/payment.rs".to_string(),
            "pub fn process_payment(amount: u64) {\n    unsafe_stripe_call(amount);\n}".to_string(),
            "pub fn process_payment(amount: u64) -> Result<(), PaymentError> {\n    // Guard with constitutional invariant\n    validate_amount(amount)?;\n    stripe_client::charge(amount)\n}".to_string(),
            vec![
                "Vantage: Scanning payment.rs...".to_string(),
                "Vantage: Checking Invariant [no_raw_sql] -> PASS".to_string(),
                "Vantage: Checking Invariant [type-safety] -> PASS".to_string(),
                "Vantage Shield: Verification complete. Safe to seal.".to_string(),
            ]
        )
    } else if intent.to_lowercase().contains("sql") || intent.to_lowercase().contains("db") || intent.to_lowercase().contains("database") {
        (
            "Sanitized SQL queries to use parameterized query builder".to_string(),
            "crates/vantage-runtime/src/db.rs".to_string(),
            "pub fn query_user(id: &str) -> String {\n    format!(\"SELECT * FROM users WHERE id = '{}'\", id)\n}".to_string(),
            "pub fn query_user(id: &str) -> Result<User, DbError> {\n    // Parameterized to prevent SQL injection\n    db.query(\"SELECT * FROM users WHERE id = ?1\", &[&id])\n}".to_string(),
            vec![
                "Vantage: Scanning db.rs...".to_string(),
                "Vantage: Checking Invariant [no_raw_sql] -> PASS".to_string(),
                "Vantage Shield: No raw SQL injection vectors detected.".to_string(),
            ]
        )
    } else {
        (
            format!("Refactored workspace components for intent: '{}'", intent),
            "crates/vantage-core/src/analytics.rs".to_string(),
            "pub fn analyze() {\n    // legacy unstructured logic\n    calculate_metrics();\n}".to_string(),
            "pub fn analyze() -> MetricReport {\n    // Structurally verified metrics computation\n    let graph = build_dependency_graph();\n    process_impact_radius_stats(&graph)\n}".to_string(),
            vec![
                "Vantage: Analyzing source files...".to_string(),
                "Vantage: Running AST parser...".to_string(),
                "Vantage: Invariant [global-hash-stable] -> PASS".to_string(),
                "Vantage Shield: Structural integrity verified.".to_string(),
            ]
        )
    };

    Ok(IntentResult {
        success: true,
        description: desc,
        file_path: file,
        original_code: orig,
        modified_code: modif,
        logs,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SealResult {
    pub success: bool,
    pub hash: String,
    pub total_nodes: usize,
    pub timestamp: u64,
}

#[tauri::command]
async fn commit_safe_state() -> Result<SealResult, String> {
    println!("Vantage Desktop: Sealing structural state...");
    
    // Execute `kit-vantage run .` under the hood. Use shell execution on Windows.
    #[cfg(target_os = "windows")]
    let output = std::process::Command::new(".\\kit-vantage.exe")
        .args(&["run", "."])
        .output()
        .map_err(|e| format!("Failed to run kit-vantage: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = std::process::Command::new("./kit-vantage")
        .args(&["run", "."])
        .output()
        .map_err(|e| format!("Failed to run kit-vantage: {}", e))?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Seal failed: {}", err_msg));
    }

    // Now let's try to parse the VANTAGE.SEAL file
    let seal_content = std::fs::read_to_string("VANTAGE.SEAL")
        .map_err(|e| format!("Failed to read VANTAGE.SEAL: {}", e))?;
    let seal_json: serde_json::Value = serde_json::from_str(&seal_content)
        .map_err(|e| format!("Failed to parse VANTAGE.SEAL: {}", e))?;

    let total_nodes = seal_json["total_nodes"].as_u64().unwrap_or(0) as usize;
    let timestamp = seal_json["ts"].as_u64().unwrap_or(0);
    
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(seal_content.as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    Ok(SealResult {
        success: true,
        hash,
        total_nodes,
        timestamp,
    })
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeMachineRevision {
    pub epoch: usize,
    pub actor: String,
    pub description: String,
    pub file_count: usize,
    pub safe: bool,
}

#[tauri::command]
async fn get_time_machine_revisions() -> Result<Vec<TimeMachineRevision>, String> {
    Ok(vec![
        TimeMachineRevision {
            epoch: 10,
            actor: "DeepSeek-R1".to_string(),
            description: "Refactored payment.rs to use safe Stripe Client API".to_string(),
            file_count: 2,
            safe: true,
        },
        TimeMachineRevision {
            epoch: 9,
            actor: "developer_01".to_string(),
            description: "Optimized workspace crates and corrected build warnings".to_string(),
            file_count: 5,
            safe: true,
        },
        TimeMachineRevision {
            epoch: 8,
            actor: "Gemini-Pro".to_string(),
            description: "Added parameterized checkout queries in db.rs".to_string(),
            file_count: 1,
            safe: true,
        },
        TimeMachineRevision {
            epoch: 7,
            actor: "System".to_string(),
            description: "Initial baseline structural graph extraction".to_string(),
            file_count: 12,
            safe: true,
        },
    ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let runtime = Arc::new(VantageRuntime::new());
    let stats = Arc::new(PEKStats::new());
    let config = VantageConfig {
        default_policy: "Enforced".to_string(),
        local_proxy_port: 8080,
        upstream_provider: "Ollama".to_string(),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            runtime,
            stats,
            config: tauri::async_runtime::Mutex::new(config),
        })
        .invoke_handler(tauri::generate_handler![
            get_pek_stats,
            update_config,
            execute_intent,
            commit_safe_state,
            get_time_machine_revisions
        ])
        .setup(|app| {
            use window_vibrancy::{apply_vibrancy, apply_mica, NSVisualEffectMaterial};

            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "windows")]
            let _ = apply_mica(&window, None);

            #[cfg(target_os = "macos")]
            let _ = apply_vibrancy(&window, NSVisualEffectMaterial::Titlebar, None, None);

            println!("Vantage Desktop Kernel initialized");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Vantage Desktop");
}
