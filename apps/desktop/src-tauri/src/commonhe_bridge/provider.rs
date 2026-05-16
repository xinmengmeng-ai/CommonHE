use std::env;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex;
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::Url;
use serde::Serialize;
use serde_json::Value as JsonValue;
use toml::Value as TomlValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderConfig {
    pub provider: String,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModel {
    pub id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCatalogEntry {
    pub provider_id: String,
    pub label: String,
    pub provider_type: String,
    pub auth_mode: String,
    pub supported_modes: Vec<String>,
    pub discovered_models: Vec<ProviderModel>,
    pub default_model: String,
    pub default_wire_api: String,
    pub requires_api_key: bool,
    pub requires_base_url: bool,
    pub default_base_url: Option<String>,
    pub supports_custom_model: bool,
    pub config_detected: bool,
    pub connectivity_validated: bool,
    pub blocking_errors: Vec<String>,
    pub user_warnings: Vec<String>,
    pub detected_sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderValidation {
    pub valid: bool,
    pub normalized_provider: String,
    pub auth_mode: String,
    pub requires_api_key: bool,
    pub requires_base_url: bool,
    pub local_configured: bool,
    pub detected_sources: Vec<String>,
    pub discovered_models: Vec<ProviderModel>,
    pub default_model: String,
    pub default_base_url: Option<String>,
    pub resolved_wire_api: String,
    pub provider_status: String,
    pub model_status: String,
    pub auth_status: String,
    pub connectivity_status: String,
    pub connectivity_validated: bool,
    pub resolved_model: Option<String>,
    pub resolved_base_url: Option<String>,
    pub blocking_errors: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub user_warnings: Vec<String>,
    pub user_facing_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalProviderStatus {
    pub provider: String,
    pub cli_available: bool,
    pub configured: bool,
    pub detected_sources: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CodexDefaults {
    pub default_model: Option<String>,
    pub default_base_url: Option<String>,
    pub wire_api: Option<String>,
}

pub fn provider_catalog() -> Vec<ProviderCatalogEntry> {
    vec![
        describe_provider("deepseek"),
        describe_provider("codex"),
        describe_provider("custom"),
        describe_provider("claude-code"),
        describe_provider("gemini-cli"),
    ]
}

pub fn describe_provider(provider: &str) -> ProviderCatalogEntry {
    let normalized_provider = normalize_provider(provider);
    let local_status = scan_local_provider_status(&normalized_provider);
    let codex_defaults = read_codex_defaults();

    match normalized_provider.as_str() {
        "deepseek" => ProviderCatalogEntry {
            provider_id: "deepseek".to_string(),
            label: "DeepSeek".to_string(),
            provider_type: "remote-api".to_string(),
            auth_mode: "api_key".to_string(),
            supported_modes: vec!["agent-chat".to_string()],
            discovered_models: vec![
                ProviderModel {
                    id: "deepseek-v4-flash".to_string(),
                    label: "deepseek-v4-flash".to_string(),
                },
                ProviderModel {
                    id: "deepseek-v4-pro".to_string(),
                    label: "deepseek-v4-pro".to_string(),
                },
            ],
            default_model: "deepseek-v4-flash".to_string(),
            default_wire_api: "chat_completions".to_string(),
            requires_api_key: true,
            requires_base_url: false,
            default_base_url: Some("https://api.deepseek.com".to_string()),
            supports_custom_model: false,
            config_detected: false,
            connectivity_validated: false,
            blocking_errors: vec![],
            user_warnings: vec![],
            detected_sources: vec![],
        },
        "codex" => {
            let auth_mode = codex_auth_mode();
            let official_login_unsupported = auth_mode == "unsupported_official_login";
            let mut discovered_models = vec![
                ProviderModel {
                    id: "gpt-5.4".to_string(),
                    label: "gpt-5.4".to_string(),
                },
                ProviderModel {
                    id: "gpt-5".to_string(),
                    label: "gpt-5".to_string(),
                },
                ProviderModel {
                    id: "gpt-5-mini".to_string(),
                    label: "gpt-5-mini".to_string(),
                },
            ];
            if let Some(model) = codex_defaults.default_model.clone() {
                push_model_if_missing(&mut discovered_models, &model);
            }

            ProviderCatalogEntry {
                provider_id: normalized_provider.clone(),
                label: "Codex".to_string(),
                provider_type: "remote-api".to_string(),
                auth_mode,
                supported_modes: vec!["agent-chat".to_string()],
                discovered_models,
                default_model: codex_defaults
                    .default_model
                    .unwrap_or_else(|| "gpt-5.4".to_string()),
                default_wire_api: codex_defaults
                    .wire_api
                    .clone()
                    .unwrap_or_else(|| "responses".to_string()),
                requires_api_key: true,
                requires_base_url: false,
                default_base_url: codex_defaults
                    .default_base_url
                    .or_else(|| Some("https://api.openai.com/v1".to_string())),
                supports_custom_model: false,
                config_detected: local_status.configured,
                connectivity_validated: false,
                blocking_errors: if official_login_unsupported {
                    vec!["codex_official_login_unsupported".to_string()]
                } else {
                    vec![]
                },
                user_warnings: local_status.warnings.clone(),
                detected_sources: local_status.detected_sources.clone(),
            }
        }
        "openai" => ProviderCatalogEntry {
            provider_id: "openai".to_string(),
            label: "OpenAI Compatible".to_string(),
            provider_type: "remote-api".to_string(),
            auth_mode: "api_key".to_string(),
            supported_modes: vec!["agent-chat".to_string()],
            discovered_models: vec![
                ProviderModel {
                    id: "gpt-5.4".to_string(),
                    label: "gpt-5.4".to_string(),
                },
                ProviderModel {
                    id: "gpt-5".to_string(),
                    label: "gpt-5".to_string(),
                },
                ProviderModel {
                    id: "gpt-5-mini".to_string(),
                    label: "gpt-5-mini".to_string(),
                },
            ],
            default_model: "gpt-5.4".to_string(),
            default_wire_api: "chat_completions".to_string(),
            requires_api_key: true,
            requires_base_url: false,
            default_base_url: Some("https://api.openai.com/v1".to_string()),
            supports_custom_model: false,
            config_detected: false,
            connectivity_validated: false,
            blocking_errors: vec![],
            user_warnings: vec![],
            detected_sources: vec![],
        },
        "custom" => ProviderCatalogEntry {
            provider_id: "custom".to_string(),
            label: "其他".to_string(),
            provider_type: "remote-api".to_string(),
            auth_mode: "api_key".to_string(),
            supported_modes: vec!["agent-chat".to_string()],
            discovered_models: vec![],
            default_model: "".to_string(),
            default_wire_api: "chat_completions".to_string(),
            requires_api_key: true,
            requires_base_url: true,
            default_base_url: None,
            supports_custom_model: true,
            config_detected: false,
            connectivity_validated: false,
            blocking_errors: vec![],
            user_warnings: vec!["requires_remote_model_discovery".to_string()],
            detected_sources: vec![],
        },
        "claude-code" => ProviderCatalogEntry {
            provider_id: "claude-code".to_string(),
            label: "Claude Code".to_string(),
            provider_type: "local-cli".to_string(),
            auth_mode: "local_cli".to_string(),
            supported_modes: vec!["detection-only".to_string()],
            discovered_models: vec![],
            default_model: "".to_string(),
            default_wire_api: "detection_only".to_string(),
            requires_api_key: false,
            requires_base_url: false,
            default_base_url: None,
            supports_custom_model: false,
            config_detected: local_status.configured,
            connectivity_validated: false,
            blocking_errors: vec!["not_in_first_wave".to_string()],
            user_warnings: vec!["first_wave_excludes_local_cli_providers".to_string()],
            detected_sources: local_status.detected_sources.clone(),
        },
        "gemini-cli" => ProviderCatalogEntry {
            provider_id: "gemini-cli".to_string(),
            label: "Gemini CLI".to_string(),
            provider_type: "local-cli".to_string(),
            auth_mode: "local_cli".to_string(),
            supported_modes: vec!["detection-only".to_string()],
            discovered_models: vec![],
            default_model: "".to_string(),
            default_wire_api: "detection_only".to_string(),
            requires_api_key: false,
            requires_base_url: false,
            default_base_url: None,
            supports_custom_model: false,
            config_detected: local_status.configured,
            connectivity_validated: false,
            blocking_errors: vec!["not_in_first_wave".to_string()],
            user_warnings: vec!["first_wave_excludes_local_cli_providers".to_string()],
            detected_sources: local_status.detected_sources.clone(),
        },
        _ => ProviderCatalogEntry {
            provider_id: normalized_provider,
            label: "未知渠道".to_string(),
            provider_type: "unknown".to_string(),
            auth_mode: "unknown".to_string(),
            supported_modes: vec![],
            discovered_models: vec![],
            default_model: "".to_string(),
            default_wire_api: "chat_completions".to_string(),
            requires_api_key: false,
            requires_base_url: false,
            default_base_url: None,
            supports_custom_model: true,
            config_detected: false,
            connectivity_validated: false,
            blocking_errors: vec!["unsupported_provider".to_string()],
            user_warnings: vec![],
            detected_sources: vec![],
        },
    }
}

pub fn validate_provider_config(config: &ProviderConfig) -> ProviderValidation {
    let normalized_provider = normalize_provider(&config.provider);
    let local_status = scan_local_provider_status(&normalized_provider);
    validate_provider_config_with_local_status(
        config,
        local_status.configured,
        local_status.detected_sources,
    )
}

pub fn validate_provider_config_with_local_status(
    config: &ProviderConfig,
    local_configured: bool,
    detected_sources: Vec<String>,
) -> ProviderValidation {
    let normalized_provider = normalize_provider(&config.provider);
    let descriptor = describe_provider(&normalized_provider);
    let codex_defaults = read_codex_defaults();
    let explicit_model = config.model.as_deref().unwrap_or("").trim();
    let resolved_model = if explicit_model.is_empty() {
        None
    } else {
        Some(explicit_model.to_string())
    };
    let api_key = resolve_api_key(&normalized_provider, config.api_key.as_deref());
    let explicit_base_url = config.base_url.as_deref().unwrap_or("").trim();
    let resolved_base_url =
        resolve_base_url(&normalized_provider, explicit_base_url, &codex_defaults);
    let resolved_wire_api = resolve_wire_api(
        &normalized_provider,
        resolved_base_url.as_deref(),
        &codex_defaults,
    );
    let effective_auth_mode = if normalized_provider == "codex" && api_key.is_some() {
        "api_key".to_string()
    } else {
        descriptor.auth_mode.clone()
    };
    let mut errors = Vec::new();
    let mut warnings = descriptor.user_warnings.clone();

    if normalized_provider.is_empty() {
        errors.push("provider_required".to_string());
    }

    for error in &descriptor.blocking_errors {
        if error == "codex_official_login_unsupported" && api_key.is_some() {
            continue;
        }
        if error == "not_in_first_wave" {
            errors.push("provider_not_in_first_wave".to_string());
        } else if error == "codex_official_login_unsupported" {
            errors.push(error.clone());
        }
    }

    if resolved_model.as_deref().unwrap_or("").is_empty() && !descriptor.supports_custom_model {
        errors.push("model_required".to_string());
    }

    if descriptor.requires_api_key && api_key.as_deref().unwrap_or("").is_empty() {
        errors.push("api_key_required".to_string());
    }

    if descriptor.requires_base_url && resolved_base_url.as_deref().unwrap_or("").is_empty() {
        errors.push("base_url_required".to_string());
    }

    if let Some(base_url) = resolved_base_url.as_deref() {
        if !is_valid_api_base_url(base_url) {
            errors.push("base_url_invalid".to_string());
        }
    }

    let model_status = if errors.iter().any(|error| error == "model_required") {
        "missing".to_string()
    } else if descriptor.discovered_models.is_empty()
        || resolved_model
            .as_deref()
            .map(|model| {
                descriptor
                    .discovered_models
                    .iter()
                    .any(|candidate| candidate.id == model)
            })
            .unwrap_or(false)
    {
        "selected".to_string()
    } else {
        warnings.push("model_not_in_discovered_list".to_string());
        "custom".to_string()
    };

    let auth_status = if effective_auth_mode == "unsupported_official_login" {
        "official_login".to_string()
    } else if descriptor.requires_api_key {
        if api_key.is_some() {
            "resolved".to_string()
        } else {
            "missing".to_string()
        }
    } else {
        "not_required".to_string()
    };
    let user_facing_error = first_user_facing_error(&errors);

    ProviderValidation {
        valid: errors.is_empty(),
        normalized_provider,
        auth_mode: effective_auth_mode,
        requires_api_key: descriptor.requires_api_key,
        requires_base_url: descriptor.requires_base_url,
        local_configured,
        detected_sources,
        discovered_models: descriptor.discovered_models,
        default_model: descriptor.default_model,
        default_base_url: descriptor.default_base_url,
        resolved_wire_api,
        provider_status: if errors.is_empty() {
            "ready_for_connectivity_check".to_string()
        } else {
            "blocked".to_string()
        },
        model_status,
        auth_status,
        connectivity_status: "not_checked".to_string(),
        connectivity_validated: false,
        resolved_model,
        resolved_base_url,
        blocking_errors: errors.clone(),
        errors,
        warnings: warnings.clone(),
        user_warnings: warnings,
        user_facing_error,
    }
}

pub fn validate_provider_connection(config: &ProviderConfig) -> ProviderValidation {
    let mut validation = validate_provider_config(config);
    if !validation.valid {
        return validation;
    }

    let Some(base_url) = validation.resolved_base_url.clone() else {
        validation.valid = false;
        validation.provider_status = "blocked".to_string();
        validation.connectivity_status = "missing_base_url".to_string();
        validation
            .blocking_errors
            .push("base_url_required".to_string());
        validation.errors = validation.blocking_errors.clone();
        return validation;
    };

    let Some(api_key) = resolve_api_key(&validation.normalized_provider, config.api_key.as_deref())
    else {
        validation.valid = false;
        validation.provider_status = "blocked".to_string();
        validation.connectivity_status = "missing_api_key".to_string();
        validation
            .blocking_errors
            .push("api_key_required".to_string());
        validation.errors = validation.blocking_errors.clone();
        return validation;
    };

    match perform_model_probe_with_base_url_candidates(
        &validation.normalized_provider,
        &base_url,
        &api_key,
        validation.resolved_model.as_deref(),
        &validation.resolved_wire_api,
    ) {
        Ok(resolved_base_url) => {
            validation.connectivity_validated = true;
            validation.connectivity_status = "validated".to_string();
            validation.provider_status = "validated".to_string();
            validation.resolved_base_url = Some(resolved_base_url);
        }
        Err(error) => {
            validation.valid = false;
            validation.connectivity_validated = false;
            validation.connectivity_status = "failed".to_string();
            validation.provider_status = "blocked".to_string();
            validation.blocking_errors.push(error.clone());
            validation.errors = validation.blocking_errors.clone();
            validation
                .user_warnings
                .push("provider_connectivity_failed".to_string());
            validation.warnings = validation.user_warnings.clone();
        }
    }

    validation
}

pub fn discover_provider_models(config: &ProviderConfig) -> Result<Vec<ProviderModel>, String> {
    let normalized_provider = normalize_provider(&config.provider);
    let codex_defaults = read_codex_defaults();
    let base_url = resolve_base_url(
        &normalized_provider,
        config.base_url.as_deref().unwrap_or("").trim(),
        &codex_defaults,
    )
    .ok_or_else(|| "base_url_required".to_string())?;

    if !is_valid_api_base_url(&base_url) {
        return Err("base_url_invalid".to_string());
    }

    let api_key = resolve_api_key(&normalized_provider, config.api_key.as_deref())
        .ok_or_else(|| "api_key_required".to_string())?;

    fetch_remote_models_with_base_url_candidates(&base_url, &api_key)
}

pub fn scan_local_providers() -> Vec<LocalProviderStatus> {
    ["codex", "claude-code", "gemini-cli"]
        .iter()
        .map(|provider| scan_local_provider_status(provider))
        .collect()
}

pub fn scan_local_provider_status(provider: &str) -> LocalProviderStatus {
    let normalized_provider = normalize_provider(provider);
    let mut detected_sources = Vec::new();
    let mut warnings = Vec::new();

    match normalized_provider.as_str() {
        "codex" => {
            detected_sources.extend(existing_env_sources(&["CODEX_API_KEY", "OPENAI_API_KEY"]));
            detected_sources.extend(existing_file_sources(&codex_config_paths()));
        }
        "claude-code" => {
            detected_sources.extend(existing_env_sources(&[
                "ANTHROPIC_API_KEY",
                "CLAUDE_CODE_OAUTH_TOKEN",
            ]));
            detected_sources.extend(existing_file_sources(&claude_config_paths()));
        }
        "gemini-cli" => {
            detected_sources.extend(existing_env_sources(&["GEMINI_API_KEY", "GOOGLE_API_KEY"]));
            detected_sources.extend(existing_file_sources(&gemini_config_paths()));
        }
        _ => {}
    }

    let cli_available = match normalized_provider.as_str() {
        "codex" => command_exists(&["codex", "codex.cmd", "codex.exe"]),
        "claude-code" => command_exists(&["claude", "claude.cmd", "claude.exe"]),
        "gemini-cli" => command_exists(&["gemini", "gemini.cmd", "gemini.exe"]),
        _ => false,
    };

    if !cli_available
        && matches!(
            normalized_provider.as_str(),
            "codex" | "claude-code" | "gemini-cli"
        )
    {
        warnings.push("cli_not_found_on_path".to_string());
    }

    LocalProviderStatus {
        provider: normalized_provider,
        cli_available,
        configured: !detected_sources.is_empty(),
        detected_sources,
        warnings,
    }
}

impl ProviderValidation {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("provider validation serialization cannot fail")
    }
}

impl LocalProviderStatus {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("local provider status serialization cannot fail")
    }

    pub fn to_json_list(statuses: &[LocalProviderStatus]) -> String {
        serde_json::to_string(statuses).expect("local provider status serialization cannot fail")
    }
}

impl ProviderCatalogEntry {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("provider descriptor serialization cannot fail")
    }

    pub fn to_json_list(catalog: &[ProviderCatalogEntry]) -> String {
        serde_json::to_string(catalog).expect("provider descriptor serialization cannot fail")
    }
}

fn normalize_provider(provider: &str) -> String {
    match provider.trim().to_lowercase().as_str() {
        "claude" | "claudecode" | "claude_code" => "claude-code".to_string(),
        "gemini" | "geminicli" | "gemini_cli" => "gemini-cli".to_string(),
        normalized => normalized.to_string(),
    }
}

fn push_model_if_missing(models: &mut Vec<ProviderModel>, model_id: &str) {
    if models.iter().any(|model| model.id == model_id) {
        return;
    }
    models.insert(
        0,
        ProviderModel {
            id: model_id.to_string(),
            label: model_id.to_string(),
        },
    );
}

fn resolve_api_key(provider: &str, explicit_api_key: Option<&str>) -> Option<String> {
    let explicit = explicit_api_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);

    if explicit.is_some() {
        return explicit;
    }

    match provider {
        "codex" => env::var("OPENAI_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                env::var("CODEX_API_KEY")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(read_codex_openai_api_key),
        "deepseek" => env::var("DEEPSEEK_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty()),
        _ => None,
    }
}

fn codex_auth_mode() -> String {
    if codex_api_key_available() {
        "api_key".to_string()
    } else if codex_official_login_available() {
        "unsupported_official_login".to_string()
    } else {
        "api_key".to_string()
    }
}

fn codex_api_key_available() -> bool {
    env::var("OPENAI_API_KEY")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .is_some()
        || env::var("CODEX_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .is_some()
        || read_codex_openai_api_key().is_some()
}

pub(crate) fn codex_official_login_available() -> bool {
    !codex_api_key_available() && read_codex_official_access_token().is_some()
}

fn read_codex_official_access_token() -> Option<String> {
    for path in codex_config_paths() {
        if path.file_name().and_then(|value| value.to_str()) != Some("auth.json") {
            continue;
        }

        if let Ok(contents) = fs::read_to_string(path) {
            if let Some(token) = parse_codex_official_access_token(&contents) {
                return Some(token);
            }
        }
    }

    None
}

pub(crate) fn resolved_api_key_for_provider(
    provider: &str,
    explicit_api_key: Option<&str>,
) -> Option<String> {
    resolve_api_key(provider, explicit_api_key)
}

pub(crate) fn resolved_wire_api_for_provider(
    provider: &str,
    explicit_base_url: Option<&str>,
) -> String {
    let codex_defaults = read_codex_defaults();
    resolve_wire_api(provider, explicit_base_url, &codex_defaults)
}

fn resolve_base_url(
    provider: &str,
    explicit_base_url: &str,
    codex_defaults: &CodexDefaults,
) -> Option<String> {
    if !explicit_base_url.trim().is_empty() {
        return Some(trim_base_url(explicit_base_url));
    }

    match provider {
        "deepseek" => Some("https://api.deepseek.com".to_string()),
        "codex" => codex_defaults
            .default_base_url
            .clone()
            .or_else(|| Some("https://api.openai.com/v1".to_string())),
        _ => None,
    }
}

fn resolve_wire_api(
    provider: &str,
    resolved_base_url: Option<&str>,
    codex_defaults: &CodexDefaults,
) -> String {
    match provider {
        "codex" => {
            let _ = resolved_base_url;
            if codex_defaults.wire_api.is_some() {
                return codex_defaults
                    .wire_api
                    .clone()
                    .unwrap_or_else(|| "chat_completions".to_string());
            }

            "chat_completions".to_string()
        }
        _ => "chat_completions".to_string(),
    }
}

fn trim_base_url(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

fn base_url_candidates(base_url: &str) -> Vec<String> {
    let primary = trim_base_url(base_url);
    if primary.is_empty() {
        return Vec::new();
    }

    let mut candidates = vec![primary.clone()];
    if let Some(v1_candidate) = v1_base_url_candidate(&primary) {
        if !candidates
            .iter()
            .any(|candidate| candidate == &v1_candidate)
        {
            candidates.push(v1_candidate);
        }
    }
    candidates
}

fn v1_base_url_candidate(base_url: &str) -> Option<String> {
    let mut url = Url::parse(base_url).ok()?;
    if url.query().is_some() || url.fragment().is_some() {
        return None;
    }

    let path = url.path().trim_end_matches('/');
    let path_lower = path.to_ascii_lowercase();
    if path_lower == "/v1"
        || path_lower.ends_with("/v1")
        || path_lower.ends_with("/models")
        || path_lower.ends_with("/responses")
        || path_lower.ends_with("/chat/completions")
    {
        return None;
    }

    let candidate_path = if path.is_empty() || path == "/" {
        "/v1".to_string()
    } else {
        format!("{path}/v1")
    };
    url.set_path(&candidate_path);
    Some(trim_base_url(url.as_str()))
}

fn is_valid_api_base_url(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };

    matches!(url.scheme(), "http" | "https") && url.has_host()
}

fn perform_model_probe_with_base_url_candidates(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: Option<&str>,
    wire_api: &str,
) -> Result<String, String> {
    let Some(model) = model.map(str::trim).filter(|model| !model.is_empty()) else {
        return Err("model_required".to_string());
    };

    let candidates = base_url_candidates(base_url);
    let mut last_model_error = None;
    for (index, candidate) in candidates.iter().enumerate() {
        match fetch_remote_models(candidate, api_key) {
            Ok(_) => {
                match wire_api {
                    "responses" => perform_responses_probe(candidate, api_key, model)?,
                    _ => perform_chat_probe(provider, candidate, api_key, model)?,
                }
                return Ok(candidate.clone());
            }
            Err(error)
                if index + 1 < candidates.len()
                    && should_retry_with_next_base_url_candidate(&error) =>
            {
                last_model_error = Some(error);
            }
            Err(error) => {
                return Err(error);
            }
        }
    }

    Err(last_model_error.unwrap_or_else(|| "provider_connectivity_failed".to_string()))
}

fn fetch_remote_models_with_base_url_candidates(
    base_url: &str,
    api_key: &str,
) -> Result<Vec<ProviderModel>, String> {
    let mut last_error = None;
    let candidates = base_url_candidates(base_url);
    for (index, candidate) in candidates.iter().enumerate() {
        match fetch_remote_models(candidate, api_key) {
            Ok(models) => return Ok(models),
            Err(error)
                if index + 1 < candidates.len()
                    && should_retry_with_next_base_url_candidate(&error) =>
            {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }

    Err(last_error.unwrap_or_else(|| "provider_connectivity_failed".to_string()))
}

fn should_retry_with_next_base_url_candidate(error: &str) -> bool {
    matches!(
        error,
        "provider_models_invalid" | "provider_models_empty" | "provider_connectivity_failed"
    )
}

fn fetch_remote_models(base_url: &str, api_key: &str) -> Result<Vec<ProviderModel>, String> {
    let models_url = format!("{}/models", trim_base_url(base_url));
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "provider_connectivity_unavailable".to_string())?;

    let response = client
        .get(models_url)
        .bearer_auth(api_key)
        .send()
        .map_err(|_| "provider_connectivity_failed".to_string())?;

    if response.status().is_success() {
        let value = response
            .json::<JsonValue>()
            .map_err(|_| "provider_models_invalid".to_string())?;
        let models = value
            .get("data")
            .and_then(JsonValue::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        let id = item.get("id").and_then(JsonValue::as_str)?.trim();
                        if id.is_empty() {
                            return None;
                        }
                        Some(ProviderModel {
                            id: id.to_string(),
                            label: id.to_string(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if models.is_empty() {
            return Err("provider_models_empty".to_string());
        }

        Ok(models)
    } else if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        Err("provider_auth_failed".to_string())
    } else {
        Err("provider_connectivity_failed".to_string())
    }
}

fn perform_chat_probe(
    provider: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
) -> Result<(), String> {
    let chat_url = format!("{}/chat/completions", trim_base_url(base_url));
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "provider_connectivity_unavailable".to_string())?;

    let response = client
        .post(chat_url)
        .bearer_auth(api_key)
        .json(&chat_probe_request_body(provider, model))
        .send()
        .map_err(|_| "provider_chat_connectivity_failed".to_string())?;

    if response.status().is_success() {
        return Ok(());
    }

    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err("provider_auth_failed".to_string());
    }

    if response.status().as_u16() == 400 || response.status().as_u16() == 404 {
        return Err("provider_model_chat_failed".to_string());
    }

    Err("provider_chat_connectivity_failed".to_string())
}

fn chat_probe_request_body(provider: &str, model: &str) -> JsonValue {
    if provider == "deepseek" {
        return serde_json::json!({
            "model": model,
            "messages": [
                { "role": "user", "content": "ping" }
            ],
            "max_tokens": 64,
            "thinking": { "type": "disabled" },
            "stream": false
        });
    }

    serde_json::json!({
        "model": model,
        "messages": [
            { "role": "user", "content": "ping" }
        ],
        "max_tokens": 4,
        "temperature": 0.0
    })
}

fn perform_responses_probe(base_url: &str, api_key: &str, model: &str) -> Result<(), String> {
    let responses_url = format!("{}/responses", trim_base_url(base_url));
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| "provider_connectivity_unavailable".to_string())?;

    let response = client
        .post(responses_url)
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": model,
            "input": "ping"
        }))
        .send()
        .map_err(|_| "provider_responses_connectivity_failed".to_string())?;

    if response.status().is_success() {
        return Ok(());
    }

    if response.status().as_u16() == 401 || response.status().as_u16() == 403 {
        return Err("provider_auth_failed".to_string());
    }

    if response.status().as_u16() == 400 || response.status().as_u16() == 404 {
        return Err("provider_model_responses_failed".to_string());
    }

    Err("provider_responses_connectivity_failed".to_string())
}

pub fn parse_codex_defaults(config: &str) -> Option<CodexDefaults> {
    let value = config.parse::<TomlValue>().ok()?;
    let default_model = value
        .get("model")
        .and_then(TomlValue::as_str)
        .map(ToString::to_string);
    let provider_name = value
        .get("model_provider")
        .and_then(TomlValue::as_str)
        .map(ToString::to_string)
        .or_else(|| infer_single_codex_model_provider(&value));

    let default_base_url = provider_name.as_ref().and_then(|provider_name| {
        value
            .get("model_providers")
            .and_then(TomlValue::as_table)
            .and_then(|providers| providers.get(provider_name))
            .and_then(TomlValue::as_table)
            .and_then(|provider| provider.get("base_url"))
            .and_then(TomlValue::as_str)
            .map(ToString::to_string)
    });
    let wire_api = provider_name.as_ref().and_then(|provider_name| {
        value
            .get("model_providers")
            .and_then(TomlValue::as_table)
            .and_then(|providers| providers.get(provider_name))
            .and_then(TomlValue::as_table)
            .and_then(|provider| provider.get("wire_api"))
            .and_then(TomlValue::as_str)
            .map(ToString::to_string)
    });

    Some(CodexDefaults {
        default_model,
        default_base_url,
        wire_api,
    })
}

fn read_codex_defaults() -> CodexDefaults {
    for path in codex_config_paths() {
        if path.file_name().and_then(|value| value.to_str()) != Some("config.toml") {
            continue;
        }

        if let Ok(contents) = fs::read_to_string(&path) {
            if let Some(defaults) = parse_codex_defaults(&contents) {
                return defaults;
            }
        }
    }

    CodexDefaults::default()
}

fn read_codex_openai_api_key() -> Option<String> {
    for path in codex_config_paths() {
        if path.file_name().and_then(|value| value.to_str()) != Some("auth.json") {
            continue;
        }

        if let Ok(contents) = fs::read_to_string(path) {
            if let Some(api_key) = parse_codex_openai_api_key(&contents) {
                return Some(api_key);
            }
        }
    }

    None
}

pub fn parse_codex_auth_json(config: &str) -> Option<String> {
    parse_codex_openai_api_key(config)
}

pub fn parse_codex_openai_api_key(config: &str) -> Option<String> {
    let value = serde_json::from_str::<JsonValue>(config).ok()?;
    value
        .get("OPENAI_API_KEY")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub fn parse_codex_official_access_token(config: &str) -> Option<String> {
    let value = serde_json::from_str::<JsonValue>(config).ok()?;
    value
        .get("tokens")
        .and_then(|tokens| tokens.get("access_token"))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn infer_single_codex_model_provider(value: &TomlValue) -> Option<String> {
    let providers = value.get("model_providers")?.as_table()?;
    if providers.len() != 1 {
        return None;
    }
    providers.keys().next().map(ToString::to_string)
}

fn existing_env_sources(names: &[&str]) -> Vec<String> {
    names
        .iter()
        .filter(|name| {
            env::var(name)
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false)
        })
        .map(|name| format!("env:{name}"))
        .collect()
}

fn existing_file_sources(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .filter(|path| path.exists())
        .map(|path| format!("file:{}", path.display()))
        .collect()
}

fn codex_config_paths() -> Vec<PathBuf> {
    if let Ok(codex_home) = env::var("CODEX_HOME") {
        let root = PathBuf::from(codex_home);
        return vec![root.join("auth.json"), root.join("config.toml")];
    }
    let mut paths = Vec::new();
    if let Some(home) = home_dir() {
        let root = home.join(".codex");
        paths.push(root.join("auth.json"));
        paths.push(root.join("config.toml"));
    }
    paths
}

fn claude_config_paths() -> Vec<PathBuf> {
    if let Some(home) = home_dir() {
        vec![
            home.join(".claude.json"),
            home.join(".claude"),
            home.join(".config").join("claude"),
        ]
    } else {
        Vec::new()
    }
}

fn gemini_config_paths() -> Vec<PathBuf> {
    if let Some(home) = home_dir() {
        vec![
            home.join(".gemini"),
            home.join(".gemini").join("oauth_creds.json"),
            home.join(".gemini").join("settings.json"),
            home.join(".config").join("gemini"),
        ]
    } else {
        Vec::new()
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn command_exists(names: &[&str]) -> bool {
    command_path_from_names(names).is_some()
}

fn command_path_from_names(names: &[&str]) -> Option<PathBuf> {
    let Some(path_value) = env::var_os("PATH") else {
        return None;
    };

    for directory in env::split_paths(&path_value) {
        for name in names {
            if let Some(candidate) = command_candidate_path(&directory, name) {
                return Some(candidate);
            }
        }
    }

    None
}

fn command_candidate_path(directory: &Path, name: &str) -> Option<PathBuf> {
    let candidate = directory.join(name);
    if candidate.is_file() {
        return Some(candidate);
    }

    #[cfg(windows)]
    {
        if Path::new(name).extension().is_none() {
            for extension in ["cmd", "exe", "bat", "ps1"] {
                let candidate = directory.join(format!("{name}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn first_user_facing_error(errors: &[String]) -> Option<String> {
    errors
        .iter()
        .find_map(|error| user_facing_error_message(error))
        .map(ToString::to_string)
}

fn user_facing_error_message(error: &str) -> Option<&'static str> {
    match error {
        "provider_required" => Some("请先选择要使用的渠道。"),
        "provider_not_in_first_wave" => Some("该渠道暂不属于第一批正式可用渠道。"),
        "model_required" => Some("请先选择模型，或显式切换到自定义模型。"),
        "api_key_required" => Some("请填写APIKey。"),
        "base_url_required" => Some("请填写 API Base URL。"),
        "base_url_invalid" => Some("请输入有效的 API Base URL。"),
        "provider_auth_failed" => Some("认证失败，请检查 APIKey 是否有效。"),
        "codex_official_login_unsupported" => {
            Some("当前不支持官方 Codex 登录授权。请选择 Codex 的 OpenAI Responses API 协议配置，并提供 APIKey。")
        }
        "provider_models_empty" => Some("模型列表为空，请确认该渠道是否支持模型发现。"),
        "provider_models_invalid" => Some("模型列表返回格式无法识别，请检查渠道接口。"),
        "provider_model_chat_failed" => {
            Some("模型连通性校验失败，请确认所选模型是否支持对话接口。")
        }
        "provider_chat_connectivity_failed" => {
            Some("模型对话请求失败，请检查网络、代理、地址或渠道服务状态。")
        }
        "provider_model_responses_failed" => {
            Some("模型连通性校验失败，请确认所选模型是否支持 Responses 接口。")
        }
        "provider_responses_connectivity_failed" => {
            Some("模型 Responses 请求失败，请检查网络、代理、地址或渠道服务状态。")
        }
        "provider_connectivity_failed" => Some("渠道连通性校验失败，请检查网络、地址或认证信息。"),
        "provider_connectivity_unavailable" => Some("当前无法发起渠道连通性校验，请稍后重试。"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        sync::Arc,
        thread,
    };

    fn with_temp_codex_home<T>(
        auth_json: Option<&str>,
        config_toml: Option<&str>,
        action: impl FnOnce() -> T,
    ) -> T {
        let _guard = crate::commonhe_bridge::test_env_lock();
        let temp_root =
            std::env::temp_dir().join(format!("commonhe-provider-test-{}", std::process::id()));
        let codex_home = temp_root.join(format!("codex-home-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&codex_home).expect("temp codex home should be created");

        if let Some(auth_json) = auth_json {
            fs::write(codex_home.join("auth.json"), auth_json)
                .expect("auth.json should be written");
        }
        if let Some(config_toml) = config_toml {
            fs::write(codex_home.join("config.toml"), config_toml)
                .expect("config.toml should be written");
        }

        let previous_codex_home = std::env::var_os("CODEX_HOME");
        unsafe {
            std::env::set_var("CODEX_HOME", &codex_home);
        }

        let result = action();

        match previous_codex_home {
            Some(previous) => unsafe {
                std::env::set_var("CODEX_HOME", previous);
            },
            None => unsafe {
                std::env::remove_var("CODEX_HOME");
            },
        }
        let _ = fs::remove_dir_all(&codex_home);
        result
    }

    #[test]
    fn provider_catalog_includes_deepseek_with_expected_models() {
        let catalog = provider_catalog();
        let deepseek = catalog
            .iter()
            .find(|provider| provider.provider_id == "deepseek")
            .expect("deepseek provider should exist");

        assert_eq!(deepseek.default_model, "deepseek-v4-flash");
        assert!(deepseek
            .discovered_models
            .iter()
            .any(|model| model.id == "deepseek-v4-flash"));
        assert!(deepseek
            .discovered_models
            .iter()
            .any(|model| model.id == "deepseek-v4-pro"));
        assert!(deepseek.requires_api_key);
        assert_eq!(
            deepseek.default_base_url.as_deref(),
            Some("https://api.deepseek.com")
        );
    }

    fn spawn_probe_server(chat_status: &str) -> String {
        spawn_probe_server_with_responses(chat_status, "ok")
    }

    fn spawn_probe_server_with_responses(chat_status: &str, responses_status: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("probe server should bind");
        let addr = listener
            .local_addr()
            .expect("probe server should expose addr");
        let chat_status = chat_status.to_string();
        let responses_status = responses_status.to_string();
        thread::spawn(move || {
            for index in 0..2 {
                let (mut stream, _) = listener.accept().expect("probe request should connect");
                let mut buffer = [0_u8; 4096];
                let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if index == 0 || path.contains("/models") {
                    ("HTTP/1.1 200 OK", r#"{"data":[{"id":"custom-model"}]}"#)
                } else if path.contains("/responses") {
                    if responses_status == "ok" {
                        ("HTTP/1.1 200 OK", r#"{"output_text":"ok"}"#)
                    } else {
                        (
                            "HTTP/1.1 500 Internal Server Error",
                            r#"{"error":"responses failed"}"#,
                        )
                    }
                } else if chat_status == "ok" {
                    (
                        "HTTP/1.1 200 OK",
                        r#"{"choices":[{"message":{"content":"ok"}}]}"#,
                    )
                } else {
                    (
                        "HTTP/1.1 500 Internal Server Error",
                        r#"{"error":"chat failed"}"#,
                    )
                };
                let response = format!(
                    "{status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("probe response should write");
            }
        });

        format!("http://{}", addr)
    }

    fn spawn_missing_v1_probe_server(paths: Arc<Mutex<Vec<String>>>, wire_api: &str) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("probe server should bind");
        let addr = listener
            .local_addr()
            .expect("probe server should expose addr");
        let wire_api = wire_api.to_string();
        thread::spawn(move || {
            for _ in 0..3 {
                let (mut stream, _) = listener.accept().expect("probe request should connect");
                let mut buffer = [0_u8; 4096];
                let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                paths
                    .lock()
                    .expect("path capture should be writable")
                    .push(path.clone());

                let (status, content_type, body) = match path.as_str() {
                    "/models" => ("HTTP/1.1 200 OK", "text/html", "<html>not an api</html>"),
                    "/v1/models" => (
                        "HTTP/1.1 200 OK",
                        "application/json",
                        r#"{"data":[{"id":"custom-model"}]}"#,
                    ),
                    "/v1/responses" if wire_api == "responses" => (
                        "HTTP/1.1 200 OK",
                        "application/json",
                        r#"{"output_text":"ok"}"#,
                    ),
                    "/v1/chat/completions" if wire_api == "chat_completions" => (
                        "HTTP/1.1 200 OK",
                        "application/json",
                        r#"{"choices":[{"message":{"content":"ok"}}]}"#,
                    ),
                    _ => (
                        "HTTP/1.1 404 Not Found",
                        "application/json",
                        r#"{"error":"not found"}"#,
                    ),
                };
                let response = format!(
                    "{status}\r\ncontent-type: {content_type}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("probe response should write");
            }
        });

        format!("http://{}", addr)
    }

    fn spawn_root_probe_server(paths: Arc<Mutex<Vec<String>>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("probe server should bind");
        let addr = listener
            .local_addr()
            .expect("probe server should expose addr");
        thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("probe request should connect");
                let mut buffer = [0_u8; 4096];
                let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                paths
                    .lock()
                    .expect("path capture should be writable")
                    .push(path.clone());

                let (status, body) = match path.as_str() {
                    "/models" => ("HTTP/1.1 200 OK", r#"{"data":[{"id":"custom-model"}]}"#),
                    "/chat/completions" => (
                        "HTTP/1.1 200 OK",
                        r#"{"choices":[{"message":{"content":"ok"}}]}"#,
                    ),
                    _ => ("HTTP/1.1 404 Not Found", r#"{"error":"not found"}"#),
                };
                let response = format!(
                    "{status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("probe response should write");
            }
        });

        format!("http://{}", addr)
    }

    fn spawn_deepseek_probe_requires_non_thinking_payload(
        captured_body: Arc<Mutex<Option<String>>>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("probe server should bind");
        let addr = listener
            .local_addr()
            .expect("probe server should expose addr");
        thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().expect("probe request should connect");
                let mut buffer = [0_u8; 8192];
                let bytes_read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..bytes_read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                let body = request.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

                let (status, response_body) = match path.as_str() {
                    "/models" => (
                        "HTTP/1.1 200 OK",
                        r#"{"data":[{"id":"deepseek-v4-flash"},{"id":"deepseek-v4-pro"}]}"#,
                    ),
                    "/chat/completions" => {
                        *captured_body
                            .lock()
                            .expect("captured body should be writable") = Some(body.clone());
                        let valid_deepseek_probe = body
                            .contains(r#""thinking":{"type":"disabled"}"#)
                            && !body.contains(r#""max_tokens":4"#)
                            && body.contains(r#""stream":false"#);
                        if valid_deepseek_probe {
                            (
                                "HTTP/1.1 200 OK",
                                r#"{"choices":[{"message":{"content":"ok"}}]}"#,
                            )
                        } else {
                            (
                                "HTTP/1.1 422 Unprocessable Entity",
                                r#"{"error":{"message":"invalid probe payload","type":"invalid_request_error"}}"#,
                            )
                        }
                    }
                    _ => ("HTTP/1.1 404 Not Found", r#"{"error":"not found"}"#),
                };
                let response = format!(
                    "{status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("probe response should write");
            }
        });

        format!("http://{}", addr)
    }

    #[test]
    fn provider_validation_requires_chat_probe_not_just_models_probe() {
        let base_url = spawn_probe_server("fail");
        let result = validate_provider_connection(&ProviderConfig {
            provider: "custom".to_string(),
            model: Some("custom-model".to_string()),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
        });

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error == "provider_chat_connectivity_failed"));
        assert!(result
            .user_warnings
            .iter()
            .any(|warning| warning == "provider_connectivity_failed"));
    }

    #[test]
    fn provider_validation_passes_when_models_and_chat_probe_pass() {
        let base_url = spawn_probe_server("ok");
        let result = validate_provider_connection(&ProviderConfig {
            provider: "custom".to_string(),
            model: Some("custom-model".to_string()),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
        });

        assert!(result.valid, "{:?}", result.errors);
        assert!(result.connectivity_validated);
    }

    #[test]
    fn provider_validation_auto_resolves_missing_v1_for_codex_responses() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_missing_v1_probe_server(paths.clone(), "responses");
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(&format!(
                r#"
model_provider = "aicodewith"
model = "custom-model"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "{base_url}"
"#
            )),
            || {
                let result = validate_provider_connection(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("custom-model".to_string()),
                    api_key: None,
                    base_url: Some(base_url.clone()),
                });

                assert!(result.valid, "{:?}", result.errors);
                assert_eq!(
                    result.resolved_base_url.as_deref(),
                    Some(format!("{base_url}/v1").as_str())
                );
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result.connectivity_validated);
            },
        );

        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/models", "/v1/models", "/v1/responses"]
        );
    }

    #[test]
    fn provider_validation_keeps_root_base_url_when_root_openai_api_works() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_root_probe_server(paths.clone());
        let result = validate_provider_connection(&ProviderConfig {
            provider: "custom".to_string(),
            model: Some("custom-model".to_string()),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url.clone()),
        });

        assert!(result.valid, "{:?}", result.errors);
        assert_eq!(result.resolved_base_url.as_deref(), Some(base_url.as_str()));
        assert!(result.connectivity_validated);
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/models", "/chat/completions"]
        );
    }

    #[test]
    fn deepseek_validation_uses_official_chat_completions_protocol_not_responses() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_missing_v1_probe_server(paths.clone(), "chat_completions");
        let result = validate_provider_connection(&ProviderConfig {
            provider: "deepseek".to_string(),
            model: Some("deepseek-v4-flash".to_string()),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
        });

        assert!(result.valid, "{:?}", result.errors);
        assert_eq!(result.resolved_wire_api, "chat_completions");
        assert!(result.connectivity_validated);
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/models", "/v1/models", "/v1/chat/completions"]
        );
    }

    #[test]
    fn deepseek_validation_chat_probe_uses_official_non_thinking_payload() {
        let captured_body = Arc::new(Mutex::new(None));
        let base_url = spawn_deepseek_probe_requires_non_thinking_payload(captured_body.clone());

        let result = validate_provider_connection(&ProviderConfig {
            provider: "deepseek".to_string(),
            model: Some("deepseek-v4-flash".to_string()),
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
        });

        assert!(result.valid, "{:?}", result.errors);
        let body = captured_body
            .lock()
            .expect("captured body should be readable")
            .clone()
            .expect("chat probe body should be captured");
        assert!(body.contains(r#""thinking":{"type":"disabled"}"#), "{body}");
        assert!(body.contains(r#""stream":false"#), "{body}");
        assert!(!body.contains(r#""max_tokens":4"#), "{body}");
    }

    #[test]
    fn model_discovery_auto_resolves_missing_v1() {
        let paths = Arc::new(Mutex::new(Vec::new()));
        let base_url = spawn_missing_v1_probe_server(paths.clone(), "chat_completions");
        let models = discover_provider_models(&ProviderConfig {
            provider: "custom".to_string(),
            model: None,
            api_key: Some("test-api-key".to_string()),
            base_url: Some(base_url),
        })
        .expect("models should be discovered from /v1 when root is not an API endpoint");

        assert_eq!(
            models,
            vec![ProviderModel {
                id: "custom-model".to_string(),
                label: "custom-model".to_string(),
            }]
        );
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/models", "/v1/models"]
        );
    }

    #[test]
    fn provider_validation_uses_resolved_responses_wire_api_for_codex() {
        let base_url = spawn_probe_server_with_responses("fail", "ok");
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(&format!(
                r#"
model_provider = "aicodewith"
model = "custom-model"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "{base_url}"
"#
            )),
            || {
                let result = validate_provider_connection(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("custom-model".to_string()),
                    api_key: None,
                    base_url: Some(base_url.clone()),
                });

                assert!(result.valid, "{:?}", result.errors);
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result.connectivity_validated);
            },
        );
    }

    #[test]
    fn codex_responses_wire_api_survives_explicit_base_url_override() {
        let explicit_base_url = spawn_probe_server_with_responses("fail", "ok");
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(
                r#"
model_provider = "aicodewith"
model = "custom-model"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "https://ai.xingmengmeng.com/v1"
"#,
            ),
            || {
                let result = validate_provider_connection(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("custom-model".to_string()),
                    api_key: None,
                    base_url: Some(explicit_base_url.clone()),
                });

                assert!(result.valid, "{:?}", result.errors);
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result.connectivity_validated);
            },
        );
    }

    #[test]
    fn codex_responses_config_does_not_change_deepseek_custom_or_openai_compatible_wire_api() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(
                r#"
model_provider = "aicodewith"
model = "gpt-5.5"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "https://ai.xingmengmeng.com/v1"
"#,
            ),
            || {
                assert_eq!(
                    resolved_wire_api_for_provider("deepseek", Some("https://api.deepseek.com/v1")),
                    "chat_completions"
                );
                assert_eq!(
                    resolved_wire_api_for_provider(
                        "custom",
                        Some("https://openai-compatible.example/v1")
                    ),
                    "chat_completions"
                );
                assert_eq!(
                    resolved_wire_api_for_provider(
                        "openai",
                        Some("https://openai-compatible.example/v1")
                    ),
                    "chat_completions"
                );

                let openai_validation = validate_provider_config(&ProviderConfig {
                    provider: "openai".to_string(),
                    model: Some("gpt-compatible".to_string()),
                    api_key: Some("test-api-key".to_string()),
                    base_url: Some("https://openai-compatible.example/v1".to_string()),
                });
                assert_eq!(openai_validation.resolved_wire_api, "chat_completions");

                let openai_descriptor = describe_provider("openai");
                assert_eq!(openai_descriptor.default_wire_api, "chat_completions");
                assert_eq!(
                    openai_descriptor.default_base_url.as_deref(),
                    Some("https://api.openai.com/v1")
                );
                assert_eq!(openai_descriptor.default_model, "gpt-5.4");
            },
        );
    }

    #[test]
    fn provider_validation_blocks_codex_when_resolved_responses_endpoint_fails() {
        let base_url = spawn_probe_server_with_responses("ok", "fail");
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(&format!(
                r#"
model_provider = "aicodewith"
model = "custom-model"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "{base_url}"
"#
            )),
            || {
                let result = validate_provider_connection(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("custom-model".to_string()),
                    api_key: None,
                    base_url: Some(base_url.clone()),
                });

                assert!(!result.valid);
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result
                    .errors
                    .iter()
                    .any(|error| error == "provider_responses_connectivity_failed"));
            },
        );
    }

    #[test]
    fn custom_provider_rejects_invalid_base_url() {
        let result = validate_provider_config_with_local_status(
            &ProviderConfig {
                provider: "custom".to_string(),
                model: Some("gpt-5".to_string()),
                api_key: Some("test-api-key".to_string()),
                base_url: Some("notaurl".to_string()),
            },
            false,
            vec![],
        );

        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|error| error == "base_url_invalid"));
        assert_eq!(
            result.user_facing_error.as_deref(),
            Some("请输入有效的 API Base URL。")
        );
    }

    #[test]
    fn parses_codex_config_defaults_from_toml() {
        let config = r#"
model_provider = "aicodewith"
model = "gpt-5.4"

[model_providers.aicodewith]
name = "aicodewith"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://ai.xingmengmeng.com/v1"
"#;

        let defaults = parse_codex_defaults(config).expect("codex defaults should parse");

        assert_eq!(defaults.default_model.as_deref(), Some("gpt-5.4"));
        assert_eq!(
            defaults.default_base_url.as_deref(),
            Some("https://ai.xingmengmeng.com/v1")
        );
        assert_eq!(defaults.wire_api.as_deref(), Some("responses"));
    }

    #[test]
    fn parses_codex_auth_key_from_json() {
        let auth_json = r#"{
  "OPENAI_API_KEY": "test-api-key-auth"
}"#;

        let api_key = parse_codex_auth_json(auth_json).expect("auth json should yield api key");

        assert_eq!(api_key, "test-api-key-auth");
    }

    #[test]
    fn parses_official_codex_oauth_access_token_from_auth_json() {
        let auth_json = r#"{
  "OPENAI_API_KEY": null,
  "tokens": {
    "access_token": "ey-official-codex-access-token",
    "refresh_token": "refresh-token"
  }
}"#;

        assert!(parse_codex_auth_json(auth_json).is_none());
        let token = parse_codex_official_access_token(auth_json)
            .expect("official Codex auth should be detectable separately");

        assert_eq!(token, "ey-official-codex-access-token");
    }

    #[test]
    fn parses_single_codex_model_provider_when_default_provider_is_omitted() {
        let config = r#"
model = "gpt-5.5"

[model_providers.aicodewith]
wire_api = "responses"
requires_openai_auth = true
base_url = "https://sub.aielove.eu.cc"
"#;

        let defaults =
            parse_codex_defaults(config).expect("single Codex model provider should be inferred");

        assert_eq!(defaults.default_model.as_deref(), Some("gpt-5.5"));
        assert_eq!(
            defaults.default_base_url.as_deref(),
            Some("https://sub.aielove.eu.cc")
        );
        assert_eq!(defaults.wire_api.as_deref(), Some("responses"));
    }

    #[test]
    fn codex_validation_accepts_auth_json_without_manual_key() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(
                r#"
model_provider = "openai"
model = "gpt-5.4"

[model_providers.openai]
base_url = "https://api.openai.com/v1"
"#,
            ),
            || {
                let result = validate_provider_config(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("gpt-5.4".to_string()),
                    api_key: None,
                    base_url: None,
                });

                assert!(result.valid);
                assert!(result
                    .detected_sources
                    .iter()
                    .any(|source| source.contains("auth.json")));
                assert!(!result.to_json().contains("test-api-key-from-auth-json"));
            },
        );
    }

    #[test]
    fn codex_validation_blocks_official_oauth_login_without_api_channel() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": null,
  "tokens": {
    "access_token": "ey-official-codex-access-token",
    "refresh_token": "refresh-token"
  }
}"#,
            ),
            Some(
                r#"
model = "gpt-5.5"

[model_providers.aicodewith]
wire_api = "responses"
requires_openai_auth = true
base_url = "https://sub.aielove.eu.cc"
"#,
            ),
            || {
                let result = validate_provider_config(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("gpt-5.5".to_string()),
                    api_key: None,
                    base_url: None,
                });

                assert!(!result.valid);
                assert_eq!(result.auth_status, "official_login");
                assert_eq!(result.auth_mode, "unsupported_official_login");
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result.resolved_base_url.is_some());
                assert!(result
                    .errors
                    .iter()
                    .any(|error| error == "codex_official_login_unsupported"));
                assert!(!result.to_json().contains("ey-official-codex-access-token"));
            },
        );
    }

    #[test]
    fn codex_official_login_catalog_is_blocked_until_api_key_channel_exists() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": null,
  "tokens": {
    "access_token": "ey-official-codex-access-token",
    "refresh_token": "refresh-token"
  }
}"#,
            ),
            Some(
                r#"
model = "gpt-5.5"

[model_providers.aicodewith]
wire_api = "responses"
requires_openai_auth = true
base_url = "https://sub.aielove.eu.cc"
"#,
            ),
            || {
                let catalog = describe_provider("codex");
                assert_eq!(catalog.provider_type, "remote-api");
                assert_eq!(catalog.auth_mode, "unsupported_official_login");
                assert_eq!(catalog.default_wire_api, "responses");
                assert!(catalog.requires_api_key);
                assert!(!catalog.requires_base_url);
                assert!(catalog.default_base_url.is_some());
                assert!(catalog
                    .blocking_errors
                    .iter()
                    .any(|error| error == "codex_official_login_unsupported"));

                let result = validate_provider_connection(&ProviderConfig {
                    provider: "codex".to_string(),
                    model: Some("gpt-5.5".to_string()),
                    api_key: None,
                    base_url: None,
                });

                assert!(!result.valid);
                assert_eq!(result.auth_status, "official_login");
                assert_eq!(result.resolved_wire_api, "responses");
                assert!(result
                    .errors
                    .iter()
                    .any(|error| error == "codex_official_login_unsupported"));
                assert!(!result.connectivity_validated);
            },
        );
    }

    #[test]
    fn codex_wire_api_resolves_from_local_config() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": "test-api-key-from-auth-json"
}"#,
            ),
            Some(
                r#"
model_provider = "aicodewith"
model = "gpt-5.4"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "https://ai.xingmengmeng.com/v1"
"#,
            ),
            || {
                assert_eq!(
                    resolved_wire_api_for_provider("codex", Some("https://ai.xingmengmeng.com/v1")),
                    "responses"
                );
            },
        );
    }

    #[test]
    fn local_provider_accepts_base_url_without_api_key() {
        let result = validate_provider_config(&ProviderConfig {
            provider: " Ollama ".to_string(),
            model: Some("llama3.1".to_string()),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
        });

        assert!(result.valid);
        assert_eq!(result.normalized_provider, "ollama");
        assert!(!result.requires_api_key);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn cloud_provider_requires_api_key_and_model() {
        let result = validate_provider_config(&ProviderConfig {
            provider: "openai".to_string(),
            model: Some(" ".to_string()),
            api_key: None,
            base_url: Some("https://api.openai.com/v1".to_string()),
        });

        assert!(!result.valid);
        assert!(result.requires_api_key);
        assert!(result
            .errors
            .iter()
            .any(|error| error == "api_key_required"));
        assert!(result.errors.iter().any(|error| error == "model_required"));
    }

    #[test]
    fn validation_json_does_not_echo_api_key() {
        let result = validate_provider_config(&ProviderConfig {
            provider: "openai".to_string(),
            model: Some("gpt-4.1".to_string()),
            api_key: Some("test-api-key-secret-value".to_string()),
            base_url: None,
        });

        let json = result.to_json();
        assert!(json.contains("\"valid\":true"));
        assert!(!json.contains("test-api-key-secret-value"));
    }

    #[test]
    fn codex_accepts_existing_local_configuration_without_manual_api_key() {
        with_temp_codex_home(
            Some(r#"{ "OPENAI_API_KEY": "test-api-key-from-auth-json" }"#),
            None,
            || {
                let result = validate_provider_config_with_local_status(
                    &ProviderConfig {
                        provider: "codex".to_string(),
                        model: Some("gpt-5".to_string()),
                        api_key: None,
                        base_url: None,
                    },
                    true,
                    vec!["env:OPENAI_API_KEY".to_string()],
                );

                assert!(result.requires_api_key);
                assert_eq!(result.auth_mode, "api_key");
                assert_eq!(result.detected_sources, vec!["env:OPENAI_API_KEY"]);
            },
        );
    }

    #[test]
    fn cli_provider_aliases_normalize_to_desktop_ids() {
        let claude = validate_provider_config_with_local_status(
            &ProviderConfig {
                provider: "claudecode".to_string(),
                model: Some("sonnet".to_string()),
                api_key: None,
                base_url: None,
            },
            true,
            vec!["file:C:\\Users\\Star\\.claude".to_string()],
        );
        let gemini = validate_provider_config_with_local_status(
            &ProviderConfig {
                provider: "geminicli".to_string(),
                model: Some("gemini-pro".to_string()),
                api_key: None,
                base_url: None,
            },
            true,
            vec!["file:C:\\Users\\Star\\.gemini".to_string()],
        );

        assert_eq!(claude.normalized_provider, "claude-code");
        assert_eq!(gemini.normalized_provider, "gemini-cli");
    }

    #[test]
    #[ignore = "requires live DeepSeek API access"]
    fn live_deepseek_provider_validation_and_model_discovery() {
        let api_key = std::env::var("COMMONHE_DEEPSEEK_TEST_KEY")
            .expect("COMMONHE_DEEPSEEK_TEST_KEY must be set for live DeepSeek tests");
        let validation = validate_provider_connection(&ProviderConfig {
            provider: "deepseek".to_string(),
            model: Some("deepseek-v4-flash".to_string()),
            api_key: Some(api_key.clone()),
            base_url: None,
        });

        assert!(
            validation.valid,
            "{}",
            validation
                .user_facing_error
                .unwrap_or_else(|| validation.errors.join(","))
        );
        assert!(validation.connectivity_validated);

        let models = discover_provider_models(&ProviderConfig {
            provider: "deepseek".to_string(),
            model: None,
            api_key: Some(api_key),
            base_url: None,
        })
        .expect("live DeepSeek models should be discoverable");

        assert!(!models.is_empty());
        assert!(models.iter().any(|model| model.id == "deepseek-v4-flash"));
    }

    #[test]
    #[ignore = "requires local Codex auth.json with valid OpenAI credentials"]
    fn live_codex_provider_validation_and_model_discovery_from_local_auth() {
        let validation = validate_provider_connection(&ProviderConfig {
            provider: "codex".to_string(),
            model: Some("gpt-5.4".to_string()),
            api_key: None,
            base_url: None,
        });

        assert!(
            validation.valid,
            "{}",
            validation
                .user_facing_error
                .unwrap_or_else(|| validation.errors.join(","))
        );
        assert!(validation.connectivity_validated);
        assert!(validation
            .detected_sources
            .iter()
            .any(|source| source.contains("auth.json") || source.contains("config.toml")));

        let models = discover_provider_models(&ProviderConfig {
            provider: "codex".to_string(),
            model: None,
            api_key: None,
            base_url: None,
        })
        .expect("live Codex models should be discoverable from local auth");

        assert!(!models.is_empty());
    }
}
