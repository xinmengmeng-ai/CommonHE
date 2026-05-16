#![recursion_limit = "256"]

pub mod commonhe_bridge;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commonhe_bridge::agent::AgentStore::new())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commonhe_bridge::commands::locate_payload,
            commonhe_bridge::commands::validate_provider_config,
            commonhe_bridge::commands::validate_provider_connection,
            commonhe_bridge::commands::list_provider_catalog,
            commonhe_bridge::commands::discover_provider_models,
            commonhe_bridge::commands::scan_local_providers,
            commonhe_bridge::commands::load_provider_tools,
            commonhe_bridge::commands::run_orchestrator_stage,
            commonhe_bridge::commands::read_status,
            commonhe_bridge::commands::open_external_url,
            commonhe_bridge::commands::create_agent_session,
            commonhe_bridge::commands::send_agent_message,
            commonhe_bridge::commands::choose_agent_solution,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run CommonHE Desktop");
}
