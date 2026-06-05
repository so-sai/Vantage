use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;
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
        .invoke_handler(tauri::generate_handler![get_pek_stats, update_config])
        .setup(|_app| {
            println!("Vantage Desktop Kernel initialized");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Vantage Desktop");
}
