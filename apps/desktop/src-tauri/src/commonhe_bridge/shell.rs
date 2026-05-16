use std::path::Path;
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrchestratorRequest {
    pub stage: String,
    pub session_root: Option<String>,
    pub project_root: Option<String>,
    pub input_text: Option<String>,
    pub choice: Option<String>,
    pub target_root: Option<String>,
    pub values_path: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub execute: bool,
    pub force: bool,
}

pub fn normalize_shell(shell: Option<&str>) -> Result<&'static str, String> {
    match shell.unwrap_or("powershell").trim().to_lowercase().as_str() {
        "" | "powershell" | "powershell.exe" => Ok("powershell"),
        "pwsh" | "pwsh.exe" => Ok("pwsh"),
        other => Err(format!("Unsupported shell: {other}")),
    }
}

pub fn validate_stage(stage: &str) -> Result<(), String> {
    let allowed = [
        "start",
        "answer",
        "propose",
        "confirm",
        "status",
        "bootstrap",
        "postcheck",
        "precheck",
        "doctor",
    ];
    if allowed.contains(&stage) {
        Ok(())
    } else {
        Err(format!("Unsupported orchestrator stage: {stage}"))
    }
}

pub fn powershell_json_command() -> &'static str {
    r#"
$ErrorActionPreference = 'Stop'
$scriptPath = $env:COMMONHE_BRIDGE_SCRIPT_PATH
$request = $env:COMMONHE_BRIDGE_REQUEST_JSON | ConvertFrom-Json
$orchestratorArgs = @{
  Stage = [string]$request.stage
}
if ($request.sessionRoot) { $orchestratorArgs.SessionRoot = [string]$request.sessionRoot }
if ($request.projectRoot) { $orchestratorArgs.ProjectRoot = [string]$request.projectRoot }
if ($request.inputText) { $orchestratorArgs.InputText = [string]$request.inputText }
if ($request.choice) { $orchestratorArgs.Choice = [string]$request.choice }
if ($request.targetRoot) { $orchestratorArgs.TargetRoot = [string]$request.targetRoot }
if ($request.valuesPath) { $orchestratorArgs.ValuesPath = [string]$request.valuesPath }
if ($request.provider) { $orchestratorArgs.Provider = [string]$request.provider }
if ($request.model) { $orchestratorArgs.Model = [string]$request.model }
if ($request.baseUrl) { $orchestratorArgs.BaseUrl = [string]$request.baseUrl }
if ($request.apiKey) { $orchestratorArgs.ApiKey = [string]$request.apiKey }
if ($request.execute) { $orchestratorArgs.Execute = $true }
if ($request.force) { $orchestratorArgs.Force = $true }
& $scriptPath @orchestratorArgs | ConvertTo-Json -Depth 80 -Compress
"#
}

pub fn powershell_args() -> Vec<String> {
    vec![
        "-NoProfile".to_string(),
        "-ExecutionPolicy".to_string(),
        "Bypass".to_string(),
        "-Command".to_string(),
        powershell_json_command().to_string(),
    ]
}

pub fn request_json(request: &OrchestratorRequest) -> String {
    serde_json::to_string(request).expect("orchestrator request serialization cannot fail")
}

pub fn run_orchestrator(
    script_path: &Path,
    request: &OrchestratorRequest,
    shell: Option<&str>,
) -> Result<String, String> {
    validate_script_path(script_path)?;
    validate_stage(&request.stage)?;
    let shell = normalize_shell(shell)?;

    let output = Command::new(shell)
        .args(powershell_args())
        .env("COMMONHE_BRIDGE_SCRIPT_PATH", script_path)
        .env("COMMONHE_BRIDGE_REQUEST_JSON", request_json(request))
        .output()
        .map_err(|error| format!("Failed to launch PowerShell bridge: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn validate_script_path(script_path: &Path) -> Result<(), String> {
    if !script_path.is_file() {
        return Err(format!(
            "Orchestrator script was not found: {}",
            script_path.display()
        ));
    }
    let file_name = script_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if file_name != "common-he-init-orchestrator.ps1" {
        return Err("Only common-he-init-orchestrator.ps1 can be executed.".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_request() -> OrchestratorRequest {
        OrchestratorRequest {
            stage: "doctor".to_string(),
            session_root: None,
            project_root: Some("E:\\WorkSoft\\Demo".to_string()),
            input_text: None,
            choice: None,
            target_root: None,
            values_path: None,
            provider: None,
            model: None,
            api_key: None,
            base_url: None,
            execute: false,
            force: false,
        }
    }

    #[test]
    fn allows_only_powershell_and_pwsh() {
        assert_eq!(normalize_shell(None).unwrap(), "powershell");
        assert_eq!(normalize_shell(Some("pwsh")).unwrap(), "pwsh");
        assert_eq!(normalize_shell(Some("powershell")).unwrap(), "powershell");
        assert!(normalize_shell(Some("cmd.exe")).is_err());
        assert!(normalize_shell(Some("bash")).is_err());
    }

    #[test]
    fn validates_orchestrator_stage_allowlist() {
        assert!(validate_stage("doctor").is_ok());
        assert!(validate_stage("bootstrap").is_ok());
        assert!(validate_stage("rm -rf").is_err());
    }

    #[test]
    fn command_forces_convert_to_json() {
        let command = powershell_json_command();

        assert!(command.contains("ConvertTo-Json"));
        assert!(command.contains("COMMONHE_BRIDGE_SCRIPT_PATH"));
        assert!(command.contains("COMMONHE_BRIDGE_REQUEST_JSON"));
    }

    #[test]
    fn request_json_serializes_only_known_fields() {
        let json = request_json(&sample_request());

        assert!(json.contains("\"stage\":\"doctor\""));
        assert!(json.contains("\"projectRoot\":\"E:\\\\WorkSoft\\\\Demo\""));
        assert!(!json.contains("script"));
    }

    #[cfg(windows)]
    #[test]
    fn run_orchestrator_passes_stage_as_named_parameter() {
        let temp =
            std::env::temp_dir().join(format!("commonhe-shell-stage-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let script_path = temp.join("common-he-init-orchestrator.ps1");
        std::fs::write(
            &script_path,
            r#"
param(
    [ValidateSet('start','doctor')]
    [string]$Stage,
    [string]$ProjectRoot
)
[pscustomobject]@{
    Stage = $Stage
    ProjectRoot = $ProjectRoot
}
"#,
        )
        .unwrap();

        let request = OrchestratorRequest {
            stage: "start".to_string(),
            session_root: None,
            project_root: Some("E:\\WorkSoft\\Demo".to_string()),
            input_text: None,
            choice: None,
            target_root: None,
            values_path: None,
            provider: None,
            model: None,
            api_key: None,
            base_url: None,
            execute: false,
            force: false,
        };

        let output = run_orchestrator(&script_path, &request, Some("powershell")).unwrap();

        assert!(output.contains("\"Stage\":\"start\""));
        assert!(output.contains("\"ProjectRoot\":\"E:\\\\WorkSoft\\\\Demo\""));
        let _ = std::fs::remove_dir_all(&temp);
    }
}
