use std::path::PathBuf;
use std::process::Command;

use tauri::{path::BaseDirectory, AppHandle, Manager};

use super::agent::{
    execute_solution_bootstrap, AgentChooseRequest, AgentSendRequest, AgentSessionCreateRequest,
    AgentStore,
};
use super::payload;
use super::provider::{self, ProviderConfig};
use super::shell::{self, OrchestratorRequest};
use super::status;

const PRODUCT_GITHUB_URL: &str = "https://github.com/xinmengmeng-ai";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalUrlCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn normalize_allowed_external_url(url: &str) -> Result<&'static str, String> {
    let normalized = url.trim().trim_end_matches('/');
    if normalized == PRODUCT_GITHUB_URL {
        Ok(PRODUCT_GITHUB_URL)
    } else {
        Err("external_url_not_allowed".to_string())
    }
}

pub fn external_url_command(url: &str) -> ExternalUrlCommand {
    #[cfg(target_os = "windows")]
    {
        ExternalUrlCommand {
            program: "explorer.exe".to_string(),
            args: vec![url.to_string()],
        }
    }

    #[cfg(target_os = "macos")]
    {
        ExternalUrlCommand {
            program: "open".to_string(),
            args: vec![url.to_string()],
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        ExternalUrlCommand {
            program: "xdg-open".to_string(),
            args: vec![url.to_string()],
        }
    }
}

pub fn spawn_external_url(url: &str) -> Result<(), String> {
    let normalized = normalize_allowed_external_url(url)?;
    let command = external_url_command(normalized);
    Command::new(&command.program)
        .args(&command.args)
        .spawn()
        .map_err(|error| format!("external_url_open_failed: {error}"))?;
    Ok(())
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), String> {
    spawn_external_url(&url)
}

#[tauri::command]
pub fn locate_payload(app: AppHandle) -> Result<String, String> {
    let resource_candidate = app
        .path()
        .resolve("resources/commonhe", BaseDirectory::Resource)
        .ok();
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let current_dir = std::env::current_dir().ok();

    let location = payload::locate_payload(
        resource_candidate.as_deref(),
        manifest_dir.as_deref(),
        current_dir.as_deref(),
    )?;

    Ok(location.to_json())
}

#[tauri::command]
pub fn validate_provider_config(
    provider: String,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<String, String> {
    let validation = provider::validate_provider_config(&ProviderConfig {
        provider,
        model,
        api_key,
        base_url,
    });

    Ok(validation.to_json())
}

#[tauri::command]
pub fn validate_provider_connection(
    provider: String,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<String, String> {
    let validation = provider::validate_provider_connection(&ProviderConfig {
        provider,
        model,
        api_key,
        base_url,
    });

    Ok(validation.to_json())
}

#[tauri::command]
pub fn list_provider_catalog() -> Result<String, String> {
    let catalog = provider::provider_catalog();
    Ok(provider::ProviderCatalogEntry::to_json_list(&catalog))
}

#[tauri::command]
pub fn discover_provider_models(
    provider: String,
    api_key: Option<String>,
    base_url: Option<String>,
) -> Result<String, String> {
    let models = provider::discover_provider_models(&ProviderConfig {
        provider,
        model: None,
        api_key,
        base_url,
    })?;

    Ok(serde_json::to_string(&models)
        .map_err(|_| "provider_models_serialize_failed".to_string())?)
}

#[tauri::command]
pub fn scan_local_providers() -> Result<String, String> {
    let statuses = provider::scan_local_providers();
    Ok(provider::LocalProviderStatus::to_json_list(&statuses))
}

#[tauri::command]
pub fn load_provider_tools(provider: String) -> Result<String, String> {
    let catalog = provider::describe_provider(&provider);
    Ok(catalog.to_json())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn run_orchestrator_stage(
    app: AppHandle,
    stage: String,
    session_root: Option<String>,
    project_root: Option<String>,
    input_text: Option<String>,
    choice: Option<String>,
    target_root: Option<String>,
    values_path: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    execute: Option<bool>,
    force: Option<bool>,
    shell: Option<String>,
) -> Result<String, String> {
    let resource_candidate = app
        .path()
        .resolve("resources/commonhe", BaseDirectory::Resource)
        .ok();
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let current_dir = std::env::current_dir().ok();
    let location = payload::locate_payload(
        resource_candidate.as_deref(),
        manifest_dir.as_deref(),
        current_dir.as_deref(),
    )?;

    shell::run_orchestrator(
        &location.orchestrator_path,
        &OrchestratorRequest {
            stage,
            session_root,
            project_root,
            input_text,
            choice,
            target_root,
            values_path,
            provider,
            model,
            api_key,
            base_url,
            execute: execute.unwrap_or(false),
            force: force.unwrap_or(false),
        },
        shell.as_deref(),
    )
}

#[tauri::command]
pub fn read_status(
    project_root: Option<String>,
    session_root: Option<String>,
) -> Result<String, String> {
    let project_root = project_root.map(PathBuf::from);
    let session_root = session_root.map(PathBuf::from);

    status::read_status_json(project_root.as_deref(), session_root.as_deref())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_agent_session(
    app: AppHandle,
    agent_store: tauri::State<'_, AgentStore>,
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
    wire_api: String,
    workspace_path: String,
) -> Result<String, String> {
    let resource_candidate = app
        .path()
        .resolve("resources/commonhe", BaseDirectory::Resource)
        .ok();
    let manifest_dir = option_env!("CARGO_MANIFEST_DIR").map(PathBuf::from);
    let current_dir = std::env::current_dir().ok();
    let location = payload::locate_payload(
        resource_candidate.as_deref(),
        manifest_dir.as_deref(),
        current_dir.as_deref(),
    )?;
    let store = agent_store.inner().clone();
    let request = AgentSessionCreateRequest {
        provider,
        model,
        api_key,
        base_url,
        wire_api,
        workspace_path,
        payload_root: location.payload_root,
        orchestrator_path: location.orchestrator_path,
    };
    let snapshot = tauri::async_runtime::spawn_blocking(move || store.create_session(request))
        .await
        .map_err(|_| "agent_session_join_failed".to_string())??;

    Ok(snapshot.to_json())
}

#[tauri::command]
pub async fn send_agent_message(
    agent_store: tauri::State<'_, AgentStore>,
    session_id: String,
    message: String,
) -> Result<String, String> {
    let store = agent_store.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || {
        store.send_message(AgentSendRequest {
            session_id,
            message,
        })
    })
    .await
    .map_err(|_| "agent_session_join_failed".to_string())??;
    Ok(snapshot.to_json())
}

#[tauri::command]
pub async fn choose_agent_solution(
    agent_store: tauri::State<'_, AgentStore>,
    session_id: String,
    solution_id: String,
    project_name: String,
    target_client: String,
    selected_capabilities: Vec<super::agent::SelectedCapability>,
) -> Result<String, String> {
    let store = agent_store.inner().clone();
    let bootstrap_session = tauri::async_runtime::spawn_blocking({
        let store = store.clone();
        let session_id = session_id.clone();
        let solution_id = solution_id.clone();
        move || {
            store.start_solution_bootstrap(AgentChooseRequest {
                session_id,
                solution_id,
                project_name,
                target_client,
                selected_capabilities,
            })
        }
    })
    .await
    .map_err(|_| "agent_session_join_failed".to_string())??;

    let bootstrap_result = tauri::async_runtime::spawn_blocking(move || {
        execute_solution_bootstrap(&bootstrap_session)
    })
    .await
    .map_err(|_| "agent_bootstrap_join_failed".to_string())??;

    let snapshot = store.complete_solution_bootstrap(&session_id, bootstrap_result)?;
    Ok(snapshot.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_url_allowlist_accepts_only_product_github() {
        assert_eq!(
            normalize_allowed_external_url("https://github.com/xinmengmeng-ai").unwrap(),
            "https://github.com/xinmengmeng-ai"
        );
        assert!(normalize_allowed_external_url("https://github.com/xinmengmeng-ai/").is_ok());
        assert!(normalize_allowed_external_url("https://evil.example.com").is_err());
        assert!(normalize_allowed_external_url("javascript:alert(1)").is_err());
    }

    #[test]
    fn external_url_command_is_argument_based() {
        let command = external_url_command("https://github.com/xinmengmeng-ai");

        assert!(
            command.program.contains("explorer")
                || command.program == "open"
                || command.program == "xdg-open"
        );
        assert_eq!(
            command.args,
            vec!["https://github.com/xinmengmeng-ai".to_string()]
        );
    }
}
