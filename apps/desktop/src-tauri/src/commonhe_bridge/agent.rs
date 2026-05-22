use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
#[cfg(test)]
use std::sync::Mutex as TestMutex;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use super::{
    provider,
    shell::{self, OrchestratorRequest},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSolution {
    pub id: String,
    pub title: String,
    pub architecture_summary: String,
    pub team_composition: Vec<String>,
    pub token_estimate: String,
    pub recommendation_text: String,
    #[serde(default)]
    pub role_rationale: HashMap<String, String>,
    #[serde(default)]
    pub omitted_role_rationale: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticReviewResult {
    pub passed: bool,
    pub blocking_issues: Vec<String>,
    pub questions_for_meng_xingxing: Vec<String>,
    pub required_repairs: Vec<String>,
    pub review_summary: String,
    pub confidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairDecision {
    pub round: usize,
    pub status: String,
    pub issues: Vec<String>,
    #[serde(default)]
    pub response_summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_solution: Option<AgentSolution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDialogueRound {
    pub round: usize,
    pub reviewer_agent: String,
    pub main_agent: String,
    pub review: SemanticReviewResult,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<RepairDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedFileEvidence {
    path: String,
    relative_path: String,
    content_preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SemanticReviewContext {
    phase: String,
    workspace_path: String,
    generated_files: Vec<String>,
    generated_file_evidence: Vec<GeneratedFileEvidence>,
    postcheck_passed: Option<bool>,
    truth_source_rules: Vec<String>,
}

impl SemanticReviewContext {
    fn pre_bootstrap(session: &AgentSession) -> Self {
        Self {
            phase: "pre_bootstrap_solution_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: Vec::new(),
            generated_file_evidence: Vec::new(),
            postcheck_passed: None,
            truth_source_rules: semantic_truth_source_rules(),
        }
    }

    fn final_package(session: &AgentSession, bootstrap_result: &AgentBootstrapResult) -> Self {
        Self {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: bootstrap_result.generated_files.clone(),
            generated_file_evidence: collect_generated_file_evidence(
                &session.workspace_path,
                &bootstrap_result.generated_files,
            ),
            postcheck_passed: Some(bootstrap_result.postcheck_passed),
            truth_source_rules: semantic_truth_source_rules(),
        }
    }
}

fn semantic_truth_source_rules() -> Vec<String> {
    vec![
        "产品主名称必须是 星星的vibecoding启动器。".to_string(),
        "梦星星必须产出三方案、团队组成、token 预估、角色选择理由与角色不选理由。".to_string(),
        "星梦梦必须检查用户原始需求、梦星星输出、用户选择、目标软件、能力状态、生成文件和真源规则。".to_string(),
        "星梦梦发现疑似遗漏、矛盾或语义跑偏时，必须回传梦星星修正或说明，不得由程序静默替代业务判断。".to_string(),
        "targetClient 只表示初始化协作包的后续接管软件与入口文件，例如 Codex 对应 AGENTS.md/.codex；它不是业务系统运行环境、部署平台或架构约束。".to_string(),
        "selectedCapabilities 只记录启动器/协作包工作流能力（superpowers、agent-browser、chrome-devtools、GitNexus、Speckit）；支付、数据库、AI 平台、知识库、部署等业务运行依赖应写入方案和实施文档，不得因未出现在 selectedCapabilities 中阻断。".to_string(),
        "生成结果必须是初始化协作包，不得暗示业务项目成品或业务代码已经完成。".to_string(),
        "生成包的 current-stage-user-checklist.md、first-sprint-contract.md、first-task-pack.md 必须是后续接手实施口径；不得残留模板腔、postcheck/bootstrap/初始化落盘等初始化收口待办。".to_string(),
        "final-acceptance.json.passed=true、postcheck 通过且会话收口后，初始化才允许显示成功。".to_string(),
    ]
}

fn collect_generated_file_evidence(
    workspace_path: &str,
    generated_files: &[String],
) -> Vec<GeneratedFileEvidence> {
    let workspace = PathBuf::from(workspace_path);
    let mut prioritized_files = generated_files.iter().collect::<Vec<_>>();
    prioritized_files.sort_by_key(|path| generated_file_evidence_priority(path));

    prioritized_files
        .into_iter()
        .take(40)
        .filter_map(|path_text| {
            let path = PathBuf::from(path_text);
            let resolved = if path.is_absolute() {
                path
            } else {
                workspace.join(path)
            };
            if !is_semantic_review_text_file(&resolved) {
                return None;
            }
            let content = fs::read_to_string(&resolved).ok()?;
            let content_preview = content.chars().take(4000).collect::<String>();
            let relative_path = resolved
                .strip_prefix(&workspace)
                .ok()
                .map(|relative| relative.to_string_lossy().to_string())
                .unwrap_or_else(|| resolved.to_string_lossy().to_string());
            Some(GeneratedFileEvidence {
                path: resolved.to_string_lossy().to_string(),
                relative_path,
                content_preview,
            })
        })
        .collect()
}

fn generated_file_evidence_priority(path_text: &str) -> usize {
    let normalized = path_text.replace('\\', "/").to_lowercase();
    let file_name = Path::new(path_text)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase();

    if matches!(file_name.as_str(), "agents.md" | "claude.md") {
        return 0;
    }
    if normalized.contains("/.codex/") || normalized.contains("/.claude/") {
        return 1;
    }
    if normalized.contains("/.agents/") {
        return 2;
    }
    if normalized.contains("/docs/workflow/current-stage-user-checklist")
        || normalized.contains("/docs/workflow/first-sprint-contract")
        || normalized.contains("/docs/workflow/first-task-pack")
    {
        return 2;
    }
    if normalized.contains("/docs/00-")
        || normalized.contains("/docs/project_context")
        || normalized.contains("/docs/skills/")
    {
        return 3;
    }
    if normalized.contains("/docs/") {
        return 4;
    }
    if normalized.contains("/.commonhe/session/final-acceptance")
        || normalized.contains("/.commonhe/session/xing-mengmeng-review")
        || normalized.contains("/.commonhe/session/meng-xingxing-output")
    {
        return 5;
    }
    if normalized.contains("/.specify/") {
        return 9;
    }
    6
}

fn is_semantic_review_text_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|extension| extension.to_str())
            .map(|extension| extension.to_ascii_lowercase())
            .as_deref(),
        Some("md") | Some("json") | Some("toml") | Some("txt") | Some("yml") | Some("yaml")
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentToolCall {
    pub tool_name: String,
    pub status: String,
    pub payload_json: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentReadiness {
    pub product_type: Option<String>,
    pub target_users: Option<String>,
    pub core_problem: Option<String>,
    pub key_features: Vec<String>,
    pub constraints: Vec<String>,
    pub summary_presented: bool,
    pub summary_confirmed: bool,
    pub missing_fields: Vec<String>,
    pub ready_for_solutions: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentBootstrapResult {
    pub status: String,
    pub workspace_path: String,
    pub generated_files: Vec<String>,
    pub handoff_path: Option<String>,
    pub postcheck_passed: bool,
    pub user_facing_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TargetClient {
    Codex,
    ClaudeCode,
}

impl TargetClient {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_lowercase().as_str() {
            "codex" => Ok(Self::Codex),
            "claude-code" | "claudecode" | "claude_code" => Ok(Self::ClaudeCode),
            _ => Err("agent_target_client_unsupported".to_string()),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::ClaudeCode => "claude-code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectedCapability {
    pub id: String,
    pub label: String,
    pub recommended: bool,
    pub selected: bool,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSessionSnapshot {
    pub session_id: String,
    pub stage: String,
    pub messages: Vec<AgentMessage>,
    pub understanding_summary: Option<String>,
    pub readiness: AgentReadiness,
    pub solutions: Vec<AgentSolution>,
    pub tool_calls: Vec<AgentToolCall>,
    pub bootstrap_result: Option<AgentBootstrapResult>,
    pub selected_solution_id: Option<String>,
    pub semantic_review_status: String,
    pub semantic_review_issues: Vec<String>,
    pub dialogue_round_count: usize,
    pub finished: bool,
}

#[derive(Debug, Clone)]
pub struct AgentSessionCreateRequest {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: String,
    pub wire_api: String,
    pub workspace_path: String,
    pub payload_root: PathBuf,
    pub orchestrator_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AgentSendRequest {
    pub session_id: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct AgentChooseRequest {
    pub session_id: String,
    pub solution_id: String,
    pub project_name: String,
    pub target_client: String,
    pub selected_capabilities: Vec<SelectedCapability>,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentSession {
    session_id: String,
    provider: String,
    model: String,
    api_key: String,
    base_url: String,
    wire_api: String,
    workspace_path: String,
    system_prompt: String,
    orchestrator_path: PathBuf,
    messages: Vec<AgentMessage>,
    understanding_summary: Option<String>,
    readiness: AgentReadiness,
    solutions: Vec<AgentSolution>,
    tool_calls: Vec<AgentToolCall>,
    bootstrap_result: Option<AgentBootstrapResult>,
    selected_solution_id: Option<String>,
    project_name: Option<String>,
    target_client: Option<TargetClient>,
    selected_capabilities: Vec<SelectedCapability>,
    semantic_review_status: String,
    semantic_review_issues: Vec<String>,
    dialogue_round_count: usize,
    finished: bool,
}

#[derive(Debug, Default, Clone)]
pub struct AgentStore {
    sessions: Arc<Mutex<HashMap<String, AgentSession>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentDecision {
    mode: String,
    assistant_message: String,
    understanding_summary: Option<String>,
    readiness: Option<AgentReadiness>,
    solutions: Option<Vec<AgentSolution>>,
}

#[derive(Debug)]
struct AgentHttpError {
    status_code: u16,
    body: String,
    endpoint: Option<String>,
}

impl AgentStore {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn create_session(
        &self,
        request: AgentSessionCreateRequest,
    ) -> Result<AgentSessionSnapshot, String> {
        let system_prompt = build_system_prompt(&request.payload_root, &request.workspace_path)?;
        let session_id = Uuid::new_v4().to_string();
        let wire_api =
            normalize_session_wire_api(&request.wire_api, &request.provider, &request.base_url);
        let resolved_api_key = resolve_session_api_key(&request.provider, &request.api_key)?;
        let mut session = AgentSession {
            session_id: session_id.clone(),
            provider: request.provider,
            model: request.model,
            api_key: resolved_api_key,
            base_url: request.base_url,
            wire_api,
            workspace_path: request.workspace_path,
            system_prompt,
            orchestrator_path: request.orchestrator_path,
            messages: Vec::new(),
            understanding_summary: None,
            readiness: AgentReadiness::default(),
            solutions: Vec::new(),
            tool_calls: Vec::new(),
            bootstrap_result: None,
            selected_solution_id: None,
            project_name: None,
            target_client: None,
            selected_capabilities: Vec::new(),
            semantic_review_status: "not_started".to_string(),
            semantic_review_issues: Vec::new(),
            dialogue_round_count: 0,
            finished: false,
        };

        let decision = match request_agent_decision_with_contract_repair(&session, None) {
            Ok(decision) => decision,
            Err(error) => {
                append_agent_runtime_diagnostic(&session, &session_id, "session", &error);
                return Err(error);
            }
        };
        apply_agent_decision(&mut session, decision)?;

        let snapshot = snapshot_from_session(&session_id, &session);
        self.sessions
            .lock()
            .map_err(|_| "agent_store_poisoned".to_string())?
            .insert(session_id, session);

        Ok(snapshot)
    }

    pub fn send_message(&self, request: AgentSendRequest) -> Result<AgentSessionSnapshot, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "agent_store_poisoned".to_string())?;
        let session = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| "agent_session_not_found".to_string())?;

        if session.finished {
            return Err("agent_session_already_finished".to_string());
        }

        let original_message_len = session.messages.len();
        session
            .messages
            .push(build_agent_message("user", request.message));

        let decision =
            match request_agent_decision_with_contract_repair(session, Some("user_message")) {
                Ok(decision) => decision,
                Err(error) => {
                    append_agent_runtime_diagnostic(
                        session,
                        &request.session_id,
                        "message",
                        &error,
                    );
                    session.messages.truncate(original_message_len);
                    return Err(error);
                }
            };
        apply_agent_decision(session, decision)?;

        Ok(snapshot_from_session(&request.session_id, session))
    }

    pub(crate) fn start_solution_bootstrap(
        &self,
        request: AgentChooseRequest,
    ) -> Result<AgentSession, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "agent_store_poisoned".to_string())?;
        let session = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| "agent_session_not_found".to_string())?;

        if !session
            .solutions
            .iter()
            .any(|solution| solution.id == request.solution_id)
        {
            return Err("agent_solution_not_found".to_string());
        }

        let project_name = normalize_project_name(&request.project_name)?;
        let target_client = TargetClient::parse(&request.target_client)?;
        let selected_capabilities = resolve_selected_capabilities(request.selected_capabilities);
        let selected_solution_id = request.solution_id.clone();
        session.selected_solution_id = Some(selected_solution_id.clone());
        normalize_selected_solution_for_package_roles(session, &selected_solution_id);
        session.project_name = Some(project_name.clone());
        session.target_client = Some(target_client.clone());
        session.selected_capabilities = selected_capabilities.clone();
        session.finished = false;
        session.bootstrap_result = None;
        session.tool_calls = vec![AgentToolCall {
            tool_name: "open_solution_selector".to_string(),
            status: "running".to_string(),
            payload_json: Some(
                json!({
                    "selectedSolutionId": selected_solution_id,
                    "projectName": project_name,
                    "targetClient": target_client.as_str(),
                    "selectedCapabilities": selected_capabilities,
                })
                .to_string(),
            ),
        }];
        session.messages.push(build_agent_message(
            "assistant",
            "星星正在根据已选方案生成初始化结果，请稍候。",
        ));

        Ok(session.clone())
    }

    pub fn complete_solution_bootstrap(
        &self,
        session_id: &str,
        result: AgentBootstrapResult,
    ) -> Result<AgentSessionSnapshot, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "agent_store_poisoned".to_string())?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| "agent_session_not_found".to_string())?;

        let success = result.status == "success" && result.postcheck_passed;
        if success {
            session.semantic_review_status = "passed".to_string();
            session.semantic_review_issues.clear();
            session.dialogue_round_count = 1;
        } else if result.user_facing_message.contains("星梦梦") {
            session.semantic_review_status = "failed".to_string();
            session.semantic_review_issues = vec![result.user_facing_message.clone()];
            session.dialogue_round_count = 1;
        }
        session.bootstrap_result = Some(result.clone());
        session.finished = success;
        session.tool_calls = vec![AgentToolCall {
            tool_name: "open_solution_selector".to_string(),
            status: if success {
                "completed".to_string()
            } else {
                "failed".to_string()
            },
            payload_json: Some(
                json!({
                    "selectedSolutionId": session.selected_solution_id,
                    "workspacePath": result.workspace_path,
                    "postcheckPassed": result.postcheck_passed,
                })
                .to_string(),
            ),
        }];
        session.messages.push(build_agent_message(
            "assistant",
            result.user_facing_message.clone(),
        ));

        Ok(snapshot_from_session(session_id, session))
    }
}

fn normalize_session_wire_api(wire_api: &str, provider: &str, base_url: &str) -> String {
    let provider_wire_api = provider::resolved_wire_api_for_provider(provider, Some(base_url));
    let trimmed = wire_api.trim();
    if trimmed == "responses" && provider_wire_api == "responses" {
        return trimmed.to_string();
    }
    if trimmed == "chat_completions" {
        return trimmed.to_string();
    }
    provider_wire_api
}

fn effective_session_wire_api(session: &AgentSession) -> String {
    if session.provider == "codex" && session_uses_codex_official_login(session) {
        return "responses".to_string();
    }
    if session.provider == "codex" && !session.api_key.trim().is_empty() {
        if session.wire_api.trim() == "responses" {
            return "responses".to_string();
        }
        let configured_wire_api =
            provider::resolved_wire_api_for_provider(&session.provider, Some(&session.base_url));
        if configured_wire_api == "responses" {
            return configured_wire_api;
        }
        return "chat_completions".to_string();
    }
    let configured_wire_api =
        provider::resolved_wire_api_for_provider(&session.provider, Some(&session.base_url));
    if configured_wire_api == "responses" {
        return configured_wire_api;
    }
    normalize_session_wire_api(&session.wire_api, &session.provider, &session.base_url)
}

impl AgentSessionSnapshot {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("agent snapshot serialization cannot fail")
    }
}

fn next_message_id() -> String {
    format!("msg_{}", Uuid::new_v4())
}

fn build_agent_message(role: &str, content: impl Into<String>) -> AgentMessage {
    let role = role.to_string();
    AgentMessage {
        role: role.clone(),
        content: content.into(),
        item_id: Some(next_message_id()),
        status: if role == "assistant" {
            Some("completed".to_string())
        } else {
            None
        },
    }
}

fn normalize_project_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("agent_project_name_required".to_string());
    }
    if trimmed
        .chars()
        .any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
    {
        return Err("agent_project_name_invalid".to_string());
    }
    Ok(trimmed.to_string())
}

fn normalize_selected_capabilities(
    capabilities: Vec<SelectedCapability>,
) -> Vec<SelectedCapability> {
    capabilities
        .into_iter()
        .filter(|capability| !capability.id.trim().is_empty())
        .map(|capability| SelectedCapability {
            id: capability.id.trim().to_string(),
            label: if capability.label.trim().is_empty() {
                capability.id.trim().to_string()
            } else {
                capability.label.trim().to_string()
            },
            recommended: capability.recommended,
            selected: capability.selected,
            status: if capability.status.trim().is_empty() {
                "pending".to_string()
            } else {
                capability.status.trim().to_string()
            },
            detail: capability.detail.trim().to_string(),
        })
        .collect()
}

fn resolve_selected_capabilities(capabilities: Vec<SelectedCapability>) -> Vec<SelectedCapability> {
    let normalized = normalize_selected_capabilities(capabilities);
    mandatory_workflow_capabilities()
        .into_iter()
        .map(|(id, default_label, default_detail)| {
            let incoming = normalized
                .iter()
                .find(|capability| capability.id.eq_ignore_ascii_case(id));
            let label = incoming
                .map(|capability| capability.label.trim())
                .filter(|label| !label.is_empty())
                .unwrap_or(default_label)
                .to_string();
            let incoming_status = incoming
                .map(|capability| capability.status.trim())
                .filter(|status| !status.is_empty());
            let should_use_fallback_status = incoming_status
                .map(|status| status.eq_ignore_ascii_case("skipped"))
                .unwrap_or(true);
            let incoming_detail = incoming
                .filter(|_| !should_use_fallback_status)
                .map(|capability| capability.detail.trim())
                .filter(|detail| !detail.is_empty());
            let status = if should_use_fallback_status {
                "fallback".to_string()
            } else {
                incoming_status.unwrap_or("fallback").to_string()
            };
            let detail = incoming_detail.map(ToString::to_string).unwrap_or_else(|| {
                format!("{default_detail}；必需能力，已记录为包内预装/内置模板能力。")
            });
            SelectedCapability {
                id: id.to_string(),
                label,
                recommended: true,
                selected: true,
                status,
                detail,
            }
        })
        .collect()
}

fn mandatory_workflow_capabilities() -> [(&'static str, &'static str, &'static str); 5] {
    [
        (
            "superpowers",
            "superpowers / required skills",
            "skill_presence: using-superpowers, test-driven-development",
        ),
        (
            "agent-browser",
            "agent-browser",
            "~/.agent-browser/config.json or ~/.roxybrowser/shortcuts.json",
        ),
        (
            "chrome-devtools",
            "chrome-devtools MCP",
            "~/.codex/config.toml or .mcp.json contains chrome-devtools",
        ),
        ("GitNexus", "GitNexus", "gitnexus --version"),
        ("Speckit", "Speckit", "specify --version"),
    ]
}

fn session_project_name(session: &AgentSession) -> Result<String, String> {
    session
        .project_name
        .as_deref()
        .map(normalize_project_name)
        .transpose()?
        .ok_or_else(|| "agent_project_name_required".to_string())
}

fn session_target_client(session: &AgentSession) -> TargetClient {
    session.target_client.clone().unwrap_or(TargetClient::Codex)
}

fn snapshot_from_session(session_id: &str, session: &AgentSession) -> AgentSessionSnapshot {
    AgentSessionSnapshot {
        session_id: session_id.to_string(),
        stage: if let Some(bootstrap_result) = &session.bootstrap_result {
            if bootstrap_result.status == "success" && bootstrap_result.postcheck_passed {
                "completed".to_string()
            } else {
                "bootstrap_failed".to_string()
            }
        } else if session.tool_calls.iter().any(|tool_call| {
            tool_call.tool_name == "open_solution_selector" && tool_call.status == "running"
        }) {
            "bootstrapping".to_string()
        } else if session.finished {
            "completed".to_string()
        } else if session.solutions.is_empty() {
            "conversation".to_string()
        } else {
            "solutions_ready".to_string()
        },
        messages: session.messages.clone(),
        understanding_summary: session.understanding_summary.clone(),
        readiness: session.readiness.clone(),
        solutions: session.solutions.clone(),
        tool_calls: build_tool_calls(session),
        bootstrap_result: session.bootstrap_result.clone(),
        selected_solution_id: session.selected_solution_id.clone(),
        semantic_review_status: session.semantic_review_status.clone(),
        semantic_review_issues: session.semantic_review_issues.clone(),
        dialogue_round_count: session.dialogue_round_count,
        finished: session.finished,
    }
}

fn append_agent_runtime_diagnostic(
    session: &AgentSession,
    session_id: &str,
    operation: &str,
    detail: &str,
) {
    let workspace_path = session.workspace_path.trim();
    if workspace_path.is_empty() {
        return;
    }

    let diagnostic_dir = PathBuf::from(workspace_path)
        .join(".commonhe")
        .join("session")
        .join(safe_session_path_segment(session_id));
    if fs::create_dir_all(&diagnostic_dir).is_err() {
        return;
    }

    let payload = json!({
        "schemaVersion": 1,
        "timestampUnixMs": current_unix_millis(),
        "operation": operation,
        "provider": session.provider,
        "model": session.model,
        "baseUrl": session.base_url,
        "wireApi": effective_session_wire_api(session),
        "messageCount": session.messages.len(),
        "readiness": {
            "productType": session.readiness.product_type,
            "targetUsers": session.readiness.target_users,
            "coreProblem": session.readiness.core_problem,
            "keyFeaturesCount": session.readiness.key_features.len(),
            "constraintsCount": session.readiness.constraints.len(),
            "summaryPresented": session.readiness.summary_presented,
            "summaryConfirmed": session.readiness.summary_confirmed,
            "missingFields": session.readiness.missing_fields,
            "readyForSolutions": session.readiness.ready_for_solutions,
        },
        "error": sanitize_agent_error_detail(detail),
    });

    let Ok(line) = serde_json::to_string(&payload) else {
        return;
    };
    let diagnostic_path = diagnostic_dir.join("runtime-diagnostics.jsonl");
    let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(diagnostic_path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn safe_session_path_segment(session_id: &str) -> String {
    let safe = session_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "unknown-session".to_string()
    } else {
        safe
    }
}

fn current_unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[derive(Clone)]
struct AgentCallLogContext {
    session_id: String,
    workspace_path: String,
    operation: String,
    provider: String,
    model: String,
    base_url: String,
    wire_api: String,
    trigger: Option<String>,
    attempt: String,
}

fn agent_operation_from_trigger(trigger: Option<&str>) -> &'static str {
    match trigger {
        None | Some("session_start") => "session",
        _ => "message",
    }
}

fn post_agent_decision_json(
    client: &Client,
    session: &AgentSession,
    trigger: Option<&str>,
    attempt: &str,
    endpoint: &str,
    request_body: &Value,
) -> Result<Value, AgentHttpError> {
    post_session_agent_json(
        client,
        session,
        agent_operation_from_trigger(trigger),
        trigger,
        attempt,
        endpoint,
        request_body,
    )
}

fn post_session_agent_json(
    client: &Client,
    session: &AgentSession,
    operation: &str,
    trigger: Option<&str>,
    attempt: &str,
    endpoint: &str,
    request_body: &Value,
) -> Result<Value, AgentHttpError> {
    let context = AgentCallLogContext {
        session_id: session.session_id.clone(),
        workspace_path: session.workspace_path.clone(),
        operation: operation.to_string(),
        provider: session.provider.clone(),
        model: session.model.clone(),
        base_url: session.base_url.clone(),
        wire_api: effective_session_wire_api(session),
        trigger: trigger.map(str::to_string),
        attempt: attempt.to_string(),
    };
    post_agent_json(
        client,
        endpoint,
        &session.api_key,
        request_body,
        Some(&context),
    )
}

fn append_agent_call_log(
    context: Option<&AgentCallLogContext>,
    endpoint: &str,
    request_body: &Value,
    status_code: Option<u16>,
    content_type: Option<&str>,
    response_body: Option<&str>,
    parsed_response: Option<&Value>,
    parse_error: Option<&str>,
    transport_error: Option<&str>,
) {
    let Some(context) = context else {
        return;
    };
    let Some(log_dir) = commonhe_runtime_log_dir() else {
        return;
    };
    if fs::create_dir_all(&log_dir).is_err() {
        return;
    }

    let response_structure = parsed_response
        .map(summarize_json_value_structure)
        .unwrap_or_else(|| {
            json!({
                "type": "unparsed",
                "textCandidateCount": 0,
            })
        });
    let payload = json!({
        "schemaVersion": 1,
        "timestampUnixMs": current_unix_millis(),
        "sessionId": safe_session_path_segment(&context.session_id),
        "workspacePath": context.workspace_path,
        "operation": context.operation,
        "provider": context.provider,
        "model": context.model,
        "baseUrl": context.base_url,
        "wireApi": context.wire_api,
        "trigger": context.trigger,
        "attempt": context.attempt,
        "endpoint": endpoint,
        "requestBody": redact_json_for_agent_log(request_body),
        "responseStatus": status_code,
        "responseContentType": content_type,
        "responseStructure": response_structure,
        "responseBodySnippet": response_body.map(|body| sanitize_agent_log_text(body, 4000)),
        "parseError": parse_error,
        "transportError": transport_error.map(|error| sanitize_agent_log_text(error, 1000)),
    });

    let Ok(line) = serde_json::to_string(&payload) else {
        return;
    };
    let log_path = log_dir.join("commonhe-agent-calls.jsonl");
    let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    else {
        return;
    };
    let _ = writeln!(file, "{line}");
}

fn commonhe_runtime_log_dir() -> Option<PathBuf> {
    if let Some(root) = commonhe_runtime_root_from_env() {
        return Some(root.join("data").join("logs"));
    }

    #[cfg(test)]
    {
        return Some(commonhe_test_runtime_root().join("data").join("logs"));
    }

    #[cfg(not(test))]
    {
        let exe_path = std::env::current_exe().ok()?;
        let runtime_root = exe_path.parent()?.to_path_buf();
        Some(runtime_root.join("data").join("logs"))
    }
}

fn commonhe_runtime_root_from_env() -> Option<PathBuf> {
    let root = std::env::var_os("COMMONHE_RUNTIME_ROOT")?;
    let root = PathBuf::from(root);
    if root.as_os_str().is_empty() {
        None
    } else {
        Some(root)
    }
}

#[cfg(test)]
fn commonhe_test_runtime_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("repo root should exist")
        .join("tmp")
        .join("desktop-main-flow")
        .join(format!("commonhe-runtime-log-root-{}", std::process::id()))
}

fn summarize_json_value_structure(value: &Value) -> Value {
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    json!({
        "type": value_type_name(value),
        "topLevelKeys": value.as_object()
            .map(|map| map.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "choiceCount": value.get("choices").and_then(Value::as_array).map(Vec::len),
        "outputCount": value.get("output").and_then(Value::as_array).map(Vec::len),
        "textCandidateCount": candidates.len(),
        "firstTextPreview": candidates
            .iter()
            .find(|text| !text.trim().is_empty())
            .map(|text| sanitize_agent_log_text(text, 1000)),
    })
}

fn redact_json_for_agent_log(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(sanitize_agent_log_text(text, 8000)),
        Value::Array(items) => Value::Array(items.iter().map(redact_json_for_agent_log).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| {
                    if key.eq_ignore_ascii_case("api_key")
                        || key.eq_ignore_ascii_case("apiKey")
                        || key.eq_ignore_ascii_case("authorization")
                    {
                        (key.clone(), Value::String("[redacted]".to_string()))
                    } else {
                        (key.clone(), redact_json_for_agent_log(value))
                    }
                })
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn sanitize_agent_log_text(detail: &str, max_chars: usize) -> String {
    let redacted = detail
        .replace("Bearer ", "Bearer [redacted] ")
        .replace("api_key", "api_key[redacted]");
    redact_inline_api_key_tokens(&redacted)
        .chars()
        .take(max_chars)
        .collect()
}

fn apply_agent_decision(session: &mut AgentSession, decision: AgentDecision) -> Result<(), String> {
    let readiness = merge_readiness(
        &session.readiness,
        decision.readiness,
        &session.messages,
        decision.understanding_summary.as_deref(),
        &session.provider,
    );
    let solutions_allowed = readiness.ready_for_solutions
        && readiness.summary_confirmed
        && readiness.missing_fields.is_empty();
    session.readiness = readiness.clone();
    session.understanding_summary = decision.understanding_summary.or_else(|| {
        if readiness.summary_presented {
            Some(build_readiness_summary(&readiness))
        } else {
            None
        }
    });

    if decision.mode == "solutions" && !solutions_allowed {
        session.messages.push(build_agent_message(
            "assistant",
            build_readiness_follow_up(&readiness),
        ));
        session.solutions.clear();
        session.tool_calls.clear();
        return Ok(());
    }

    session.messages.push(build_agent_message(
        "assistant",
        normalize_assistant_message(&decision.assistant_message, &readiness),
    ));

    if decision.mode == "solutions" {
        let solutions = decision
            .solutions
            .ok_or_else(|| "agent_solutions_missing".to_string())?;
        if solutions.len() != 3 {
            return Err("agent_solutions_must_be_three".to_string());
        }
        validate_solution_rationale(&solutions)?;
        session.solutions = solutions;
        session.tool_calls = vec![AgentToolCall {
            tool_name: "open_solution_selector".to_string(),
            status: "requested".to_string(),
            payload_json: Some(json!({ "solutionCount": session.solutions.len() }).to_string()),
        }];
    }

    Ok(())
}

fn resolve_session_api_key(provider_name: &str, explicit_api_key: &str) -> Result<String, String> {
    if provider_name == "codex"
        && explicit_api_key.trim().is_empty()
        && provider::codex_official_login_available()
    {
        return Err("codex_official_login_unsupported".to_string());
    }

    provider::resolved_api_key_for_provider(
        provider_name,
        if explicit_api_key.trim().is_empty() {
            None
        } else {
            Some(explicit_api_key)
        },
    )
    .ok_or_else(|| "agent_auth_failed".to_string())
}

fn request_agent_decision(
    session: &AgentSession,
    trigger: Option<&str>,
) -> Result<AgentDecision, String> {
    if session_uses_codex_official_login(session) {
        return Err("codex_official_login_unsupported".to_string());
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(agent_request_timeout(session))
        .build()
        .map_err(|_| "agent_client_build_failed".to_string())?;

    let wire_api = effective_session_wire_api(session);
    let endpoint = match wire_api.as_str() {
        "responses" => format!("{}/responses", session.base_url.trim_end_matches('/')),
        _ => format!(
            "{}/chat/completions",
            session.base_url.trim_end_matches('/')
        ),
    };

    if wire_api == "responses" {
        let primary_request = build_responses_request_body(session, trigger);
        match post_agent_decision_json(
            &client,
            session,
            trigger,
            "agent.responses.primary",
            &endpoint,
            &primary_request,
        ) {
            Ok(value) => {
                return parse_agent_decision_response_for_provider(&value, &session.provider)
            }
            Err(error) if error.status_code == 400 => {
                let compat_request = build_responses_compat_request_body(session, trigger);
                match post_agent_decision_json(
                    &client,
                    session,
                    trigger,
                    "agent.responses.compat",
                    &endpoint,
                    &compat_request,
                ) {
                    Ok(value) => {
                        return parse_agent_decision_response_for_provider(
                            &value,
                            &session.provider,
                        )
                    }
                    Err(fallback_error) => return Err(map_agent_http_error(&fallback_error)),
                }
            }
            Err(error) => return Err(map_agent_http_error(&error)),
        }
    }

    let request_body = build_chat_completions_request_body(session, trigger);
    let compat_request_body = build_chat_completions_compat_request_body(session, trigger);
    let minimal_request_body = build_chat_completions_minimal_request_body(session, trigger);
    let value = match post_agent_decision_json(
        &client,
        session,
        trigger,
        "agent.chat.primary",
        &endpoint,
        &request_body,
    ) {
        Ok(value) => value,
        Err(error) if error.status_code == 400 => {
            match post_agent_decision_json(
                &client,
                session,
                trigger,
                "agent.chat.compat",
                &endpoint,
                &compat_request_body,
            ) {
                Ok(value) => value,
                Err(compat_error)
                    if compat_error.status_code == 400
                        || should_retry_chat_request_with_compat_body(session, &compat_error) =>
                {
                    post_agent_decision_json(
                        &client,
                        session,
                        trigger,
                        "agent.chat.minimal",
                        &endpoint,
                        &minimal_request_body,
                    )
                    .map_err(|minimal_error| map_agent_http_error(&minimal_error))?
                }
                Err(compat_error) => return Err(map_agent_http_error(&compat_error)),
            }
        }
        Err(error) if should_retry_chat_request_with_compat_body(session, &error) => {
            match post_agent_decision_json(
                &client,
                session,
                trigger,
                "agent.chat.compat",
                &endpoint,
                &compat_request_body,
            ) {
                Ok(value) => value,
                Err(compat_error)
                    if compat_error.status_code == 400
                        || should_retry_chat_request_with_compat_body(session, &compat_error) =>
                {
                    post_agent_decision_json(
                        &client,
                        session,
                        trigger,
                        "agent.chat.minimal",
                        &endpoint,
                        &minimal_request_body,
                    )
                    .map_err(|minimal_error| map_agent_http_error(&minimal_error))?
                }
                Err(compat_error) => return Err(map_agent_http_error(&compat_error)),
            }
        }
        Err(error) if should_retry_chat_request_as_responses(session, &error) => {
            request_agent_decision_via_responses(&client, session, trigger, Some(error))?
        }
        Err(error) => return Err(map_agent_http_error(&error)),
    };

    match parse_agent_decision_response_for_provider(&value, &session.provider) {
        Ok(decision) => Ok(decision),
        Err(error) if should_retry_agent_parse_with_chat_fallback(&error) => {
            let compat_value = match post_agent_decision_json(
                &client,
                session,
                trigger,
                "agent.chat.compat",
                &endpoint,
                &compat_request_body,
            ) {
                Ok(value) => value,
                Err(compat_error)
                    if compat_error.status_code == 400
                        || should_retry_chat_request_with_compat_body(session, &compat_error) =>
                {
                    post_agent_decision_json(
                        &client,
                        session,
                        trigger,
                        "agent.chat.minimal",
                        &endpoint,
                        &minimal_request_body,
                    )
                    .map_err(|minimal_error| map_agent_http_error(&minimal_error))?
                }
                Err(compat_error) => return Err(map_agent_http_error(&compat_error)),
            };
            match parse_agent_decision_response_for_provider(&compat_value, &session.provider) {
                Ok(decision) => Ok(decision),
                Err(compat_parse_error)
                    if should_retry_agent_parse_with_chat_fallback(&compat_parse_error) =>
                {
                    let minimal_value = post_agent_decision_json(
                        &client,
                        session,
                        trigger,
                        "agent.chat.minimal",
                        &endpoint,
                        &minimal_request_body,
                    )
                    .map_err(|minimal_error| map_agent_http_error(&minimal_error))?;
                    parse_agent_decision_response_for_provider(&minimal_value, &session.provider)
                }
                Err(compat_parse_error) => Err(compat_parse_error),
            }
        }
        Err(error) => Err(error),
    }
}

fn request_agent_decision_via_responses(
    client: &Client,
    session: &AgentSession,
    trigger: Option<&str>,
    _previous_error: Option<AgentHttpError>,
) -> Result<Value, String> {
    let endpoint = format!("{}/responses", session.base_url.trim_end_matches('/'));
    let primary_request = build_responses_request_body(session, trigger);
    match post_agent_decision_json(
        client,
        session,
        trigger,
        "agent.responses.retry.primary",
        &endpoint,
        &primary_request,
    ) {
        Ok(value) => Ok(value),
        Err(error) if error.status_code == 400 => {
            let compat_request = build_responses_compat_request_body(session, trigger);
            post_agent_decision_json(
                client,
                session,
                trigger,
                "agent.responses.retry.compat",
                &endpoint,
                &compat_request,
            )
            .map_err(|fallback_error| map_agent_http_error(&fallback_error))
        }
        Err(error) => Err(map_agent_http_error(&error)),
    }
}

fn should_retry_chat_request_as_responses(session: &AgentSession, error: &AgentHttpError) -> bool {
    session.provider.trim().eq_ignore_ascii_case("codex")
        && session.base_url.trim_end_matches('/') != "https://api.openai.com/v1"
        && (error.status_code == 0 || error.status_code == 404 || error.status_code >= 500)
}

fn should_retry_chat_request_with_compat_body(
    session: &AgentSession,
    error: &AgentHttpError,
) -> bool {
    session.provider.trim().eq_ignore_ascii_case("deepseek")
        && error.status_code == 200
        && ((error.body.starts_with("invalid_json") && error.body.contains("reason=empty_body"))
            || error.body.starts_with("response_body_read_failed"))
}

const MAX_AGENT_CONTRACT_REPAIR_ROUNDS: usize = 2;

fn request_agent_decision_with_contract_repair(
    session: &AgentSession,
    trigger: Option<&str>,
) -> Result<AgentDecision, String> {
    let mut repair_rounds = 0usize;
    let mut next_trigger = trigger;

    loop {
        match request_agent_decision(session, next_trigger) {
            Ok(decision) if should_retry_confirmed_question_as_solutions(session, &decision) => {
                if repair_rounds < MAX_AGENT_CONTRACT_REPAIR_ROUNDS {
                    repair_rounds += 1;
                    next_trigger = Some("contract_repair_confirmed_solutions");
                    continue;
                }

                return Ok(decision);
            }
            Ok(decision) => return Ok(decision),
            Err(error)
                if is_repairable_agent_contract_error(&error)
                    && repair_rounds < MAX_AGENT_CONTRACT_REPAIR_ROUNDS =>
            {
                repair_rounds += 1;
                next_trigger = Some(contract_repair_trigger(&error));
            }
            Err(error) => return Err(error),
        }
    }
}

fn should_retry_confirmed_question_as_solutions(
    session: &AgentSession,
    decision: &AgentDecision,
) -> bool {
    if decision.mode != "question" {
        return false;
    }

    let Some(latest_user) = latest_user_message(&session.messages) else {
        return false;
    };

    is_explicit_confirmation_message_for_provider(&session.provider, latest_user)
        && readiness_has_required_details(&session.readiness)
        && session.readiness.summary_presented
        && session
            .readiness
            .missing_fields
            .iter()
            .all(|field| field == "用户确认")
}

fn agent_request_timeout(session: &AgentSession) -> Duration {
    if provider_allows_minimax_reasoning_cleanup(&session.provider)
        || (session.provider.trim().eq_ignore_ascii_case("deepseek")
            && session.model.trim().eq_ignore_ascii_case("deepseek-v4-pro"))
    {
        Duration::from_secs(180)
    } else {
        Duration::from_secs(60)
    }
}

fn build_agent_instruction() -> &'static str {
    "Collect enough information to understand product type, target users, core problem, key features, and constraints. Do not return solutions until those fields are complete and the user has explicitly confirmed your summary is accurate. When information is missing, return mode=question and ask exactly one concise follow-up question. When all required information is complete and the user has confirmed your summary, return mode=solutions with exactly three solutions. Solutions are implementation plans for the later handoff, not claims that business code or a finished MVP has already been generated. tokenEstimate must describe LLM planning/review/handoff budget, not completed business code. omittedRoleRationale must only include roles that are not in teamComposition. If a target client such as Codex or Claude Code is later selected, treat it only as the collaboration-package handoff entry, not as the business runtime, hosting platform, deployment platform, or architecture requirement."
}

fn agent_json_contract_instruction() -> &'static str {
    "Return exactly one complete, valid JSON object and nothing else. Hard requirements: Output must be raw JSON only. Do not include Markdown, bullet lists, XML/HTML tags, YAML, code fences, commentary, explanatory prose, or any text before or after the JSON object. Do not include pseudo tool calls or agent tags such as <solution-picker-agent>. Ensure the JSON is syntactically valid and fully parseable by a desktop program. Include mode with the exact string value \"question\" or \"solutions\". Include readiness as a JSON object when reporting discovery state. If mode is \"solutions\", include solutions as a JSON array at the top level; do not omit mode or solutions[]. Minimum acceptance criteria: The response parses as JSON without errors. The top-level object contains \"mode\". If and only if mode is \"solutions\", the top-level object contains \"mode\": \"solutions\" and \"solutions\": [] or a populated solutions array. The desktop program opens the solution selector only after it successfully parses mode=\"solutions\" and solutions[]."
}

fn build_agent_context_prompt(session: &AgentSession, trigger: Option<&str>) -> String {
    format!(
        "Current provider: {}. Current workspace: {}. Trigger: {}. Important product semantics: target clients such as Codex and Claude Code only select the generated collaboration package entry files; they are not business runtime, deployment, hosting, or architecture platforms. {} {} {} {} Respond with JSON only. JSON shape: {{\"mode\":\"question|solutions\",\"assistantMessage\":\"...\",\"understandingSummary\":\"...\",\"readiness\":{{\"productType\":\"...\",\"targetUsers\":\"...\",\"coreProblem\":\"...\",\"keyFeatures\":[\"...\"],\"constraints\":[\"...\"],\"summaryPresented\":true,\"summaryConfirmed\":false,\"missingFields\":[\"...\"],\"readyForSolutions\":false}},\"solutions\":[{{\"id\":\"A\",\"title\":\"...\",\"architectureSummary\":\"...\",\"teamComposition\":[\"...\"],\"tokenEstimate\":\"...\",\"recommendationText\":\"...\",\"roleRationale\":{{\"frontend\":\"why this role is needed\"}},\"omittedRoleRationale\":{{\"qa\":\"why this plausible role is not selected yet\"}}}}]}}",
        session.provider,
        session.workspace_path,
        trigger.unwrap_or("session_start"),
        build_agent_instruction(),
        agent_json_contract_instruction(),
        agent_contract_repair_instruction(trigger),
        custom_provider_agent_context(session)
    )
}

fn custom_provider_agent_context(session: &AgentSession) -> String {
    if !provider_allows_minimax_reasoning_cleanup(&session.provider) {
        return String::new();
    }

    let latest_user = latest_user_message(&session.messages).unwrap_or_default();
    let latest_user_confirmed =
        is_explicit_confirmation_message_for_provider(&session.provider, latest_user);
    let readiness = &session.readiness;
    let missing = if readiness.missing_fields.is_empty() {
        "none".to_string()
    } else {
        readiness.missing_fields.join(",")
    };

    format!(
        "Custom provider state: currentReadiness={{productType:{},targetUsers:{},coreProblem:{},keyFeatures:{},constraints:{},summaryPresented:{},summaryConfirmed:{},missingFields:{}}}; latestUserConfirmed={}. If latestUserConfirmed=true and all required detail fields are present, output mode=solutions with exactly three alternatives now. Do not ask optional follow-up questions such as priority or tenancy; place those as assumptions/trade-offs inside the alternatives.",
        readiness.product_type.as_deref().unwrap_or(""),
        readiness.target_users.as_deref().unwrap_or(""),
        readiness.core_problem.as_deref().unwrap_or(""),
        readiness.key_features.join("|"),
        readiness.constraints.join("|"),
        readiness.summary_presented,
        readiness.summary_confirmed,
        missing,
        latest_user_confirmed
    )
}

fn contract_repair_trigger(error: &str) -> &'static str {
    match error {
        "agent_response_not_json" => "contract_repair",
        "agent_solution_role_rationale_missing" => "contract_repair_missing_role_rationale",
        "agent_solutions_unstructured_markdown" => "contract_repair_unstructured_solutions",
        _ => "contract_repair",
    }
}

fn is_repairable_agent_contract_error(error: &str) -> bool {
    matches!(
        error,
        "agent_response_not_json"
            | "agent_solution_role_rationale_missing"
            | "agent_solutions_missing"
            | "agent_solutions_must_be_three"
            | "agent_solutions_unstructured_markdown"
    )
}

fn is_immediate_agent_contract_error(error: &str) -> bool {
    matches!(
        error,
        "agent_solution_role_rationale_missing"
            | "agent_solutions_missing"
            | "agent_solutions_must_be_three"
            | "agent_solutions_unstructured_markdown"
    )
}

fn agent_contract_repair_instruction(trigger: Option<&str>) -> &'static str {
    match trigger {
        Some("contract_repair_missing_role_rationale") => {
            "Your previous solutions response was rejected because at least one solution missed roleRationale or omittedRoleRationale. Do not ask the user to repeat anything. Reuse the existing conversation context and output exactly three complete solutions now, each with roleRationale and omittedRoleRationale."
        }
        Some("contract_repair_unstructured_solutions") => {
            "Your previous response described 方案A/方案B/方案C in Markdown instead of returning the required structured JSON. Do not ask the user to choose in chat. Do not output <solution-picker-agent> or any other pseudo tool call; the desktop program opens the solution selector after parsing solutions[]. Reuse the existing conversation context and output JSON only. Return mode=solutions and solutions[] now. Follow this contract exactly: Return exactly one complete, valid JSON object and nothing else. Hard requirements: Output must be raw JSON only. Do not include Markdown, bullet lists, XML/HTML tags, YAML, code fences, commentary, explanatory prose, or any text before or after the JSON object. Do not include pseudo tool calls or agent tags such as <solution-picker-agent>. Ensure the JSON is syntactically valid and fully parseable by a desktop program. Include mode with the exact string value \"solutions\". Include solutions as a JSON array at the top level. Do not omit mode or solutions[]. Minimum acceptance criteria: The response parses as JSON without errors. The top-level object contains \"mode\": \"solutions\" and \"solutions\": [] or a populated solutions array. Exactly three solution items are required, and every solution must include architectureSummary, teamComposition, tokenEstimate, recommendationText, roleRationale, and omittedRoleRationale."
        }
        Some("contract_repair") => {
            "Your previous response did not satisfy the required JSON contract. Do not ask the user to repeat anything. Reuse the existing conversation context and output the corrected JSON now. Return exactly one complete, valid JSON object and nothing else. Hard requirements: Output must be raw JSON only. Do not include Markdown, bullet lists, XML/HTML tags, YAML, code fences, commentary, explanatory prose, or any text before or after the JSON object. Do not include pseudo tool calls or agent tags such as <solution-picker-agent>. Ensure the JSON is syntactically valid and fully parseable by a desktop program. Include mode with the exact string value \"question\" or \"solutions\". Include readiness when reporting discovery state. If mode is \"solutions\", include solutions as a JSON array at the top level. Do not omit mode or solutions[] when mode is \"solutions\". Minimum acceptance criteria: The response parses as JSON without errors. The top-level object contains \"mode\". If mode is \"solutions\", the top-level object contains \"mode\": \"solutions\" and \"solutions\": [] or a populated solutions array."
        }
        Some("contract_repair_confirmed_solutions") => {
            "The user has explicitly confirmed the presented understanding, and the required product fields are already present. Do not ask new optional scoping questions. Encode assumptions and trade-offs inside the three alternatives. Respond with JSON only: mode=solutions and exactly three solutions, each with architectureSummary, teamComposition, tokenEstimate, recommendationText, roleRationale, and omittedRoleRationale. Follow this contract exactly: Return exactly one complete, valid JSON object and nothing else. Output must be raw JSON only. Do not include Markdown, bullet lists, XML/HTML tags, YAML, code fences, commentary, explanatory prose, pseudo tool calls, or agent tags such as <solution-picker-agent>."
        }
        _ => "",
    }
}

fn build_chat_completions_request_body(session: &AgentSession, trigger: Option<&str>) -> Value {
    let messages = build_chat_completions_messages(session, trigger);

    with_deepseek_chat_safety_parameters(
        session,
        json!({
            "model": session.model,
            "messages": messages,
            "temperature": 0.4,
            "response_format": { "type": "json_object" }
        }),
    )
}

fn build_chat_completions_compat_request_body(
    session: &AgentSession,
    trigger: Option<&str>,
) -> Value {
    with_deepseek_chat_safety_parameters(
        session,
        json!({
            "model": session.model,
            "messages": build_chat_completions_messages(session, trigger),
        }),
    )
}

fn build_chat_completions_minimal_request_body(
    session: &AgentSession,
    trigger: Option<&str>,
) -> Value {
    let mut messages = vec![json!({
        "role": "system",
        "content": build_minimal_agent_prompt(session, trigger),
    })];

    if session.messages.is_empty() {
        messages.push(json!({
            "role": "user",
            "content": "请先用一句话说明你正在为星星的vibecoding启动器做初始化需求澄清，然后提出一个最关键的追问。"
        }));
    } else {
        for message in &session.messages {
            messages.push(json!({
                "role": message.role,
                "content": message.content,
            }));
        }
    }

    with_deepseek_chat_safety_parameters(
        session,
        json!({
            "model": session.model,
            "messages": messages,
        }),
    )
}

fn with_deepseek_chat_safety_parameters(session: &AgentSession, mut body: Value) -> Value {
    if session.provider != "deepseek" {
        return body;
    }

    if let Some(map) = body.as_object_mut() {
        map.entry("max_tokens".to_string()).or_insert(json!(4096));
        map.entry("thinking".to_string())
            .or_insert(json!({ "type": "disabled" }));
        map.entry("stream".to_string()).or_insert(json!(false));
    }

    body
}

fn build_chat_completions_messages(session: &AgentSession, trigger: Option<&str>) -> Vec<Value> {
    let mut messages = vec![
        json!({
            "role": "system",
            "content": session.system_prompt,
        }),
        json!({
            "role": "system",
            "content": build_agent_context_prompt(session, trigger),
        }),
    ];

    for message in &session.messages {
        messages.push(json!({
            "role": message.role,
            "content": message.content,
        }));
    }

    if session.messages.is_empty() {
        messages.push(json!({
            "role": "user",
            "content": "请先用一句话说明你正在为星星的vibecoding启动器做初始化需求澄清，然后提出一个最关键的追问。"
        }));
    }

    messages
}

fn build_minimal_agent_prompt(session: &AgentSession, trigger: Option<&str>) -> String {
    format!(
        "You are 梦星星, the main initialization agent for 星星的vibecoding启动器. Current provider: {}. Current workspace: {}. Trigger: {}. Understand the user's product idea first, ask natural follow-up questions, and do not output solutions until product type, target users, core problem, key features, and constraints are complete and the user explicitly confirms your summary. After confirmation, output exactly three solutions. Each solution must include roleRationale and omittedRoleRationale so 星梦梦 can review your role choices. Solutions are implementation plans for a later handoff; do not claim the generated init package already contains finished business code. Codex or Claude Code are only handoff clients, not deployment or runtime platforms. {} Respond with JSON only using this shape: {{\"mode\":\"question|solutions\",\"assistantMessage\":\"...\",\"understandingSummary\":\"...\",\"readiness\":{{\"productType\":\"...\",\"targetUsers\":\"...\",\"coreProblem\":\"...\",\"keyFeatures\":[\"...\"],\"constraints\":[\"...\"],\"summaryPresented\":true,\"summaryConfirmed\":false,\"missingFields\":[\"...\"],\"readyForSolutions\":false}},\"solutions\":[{{\"id\":\"A\",\"title\":\"...\",\"architectureSummary\":\"...\",\"teamComposition\":[\"...\"],\"tokenEstimate\":\"...\",\"recommendationText\":\"...\",\"roleRationale\":{{\"frontend\":\"why this role is needed\"}},\"omittedRoleRationale\":{{\"qa\":\"why this plausible role is not selected yet\"}}}}]}}",
        session.provider,
        session.workspace_path,
        trigger.unwrap_or("session_start"),
        agent_json_contract_instruction(),
    )
}

fn build_responses_request_body(session: &AgentSession, trigger: Option<&str>) -> Value {
    let input = if session.messages.is_empty() {
        vec![json!({
            "type": "message",
            "role": "user",
            "id": next_message_id(),
            "content": [
                {
                    "type": "input_text",
                    "text": "请先用一句话说明你正在为星星的vibecoding启动器做初始化需求澄清，然后提出一个最关键的追问。"
                }
            ]
        })]
    } else {
        session
            .messages
            .iter()
            .map(|message| {
                json!({
                    "type": "message",
                    "role": message.role,
                    "id": message.item_id.clone().unwrap_or_else(next_message_id),
                    "status": message.status,
                    "content": [
                        {
                            "type": "input_text",
                            "text": message.content
                        }
                    ]
                })
            })
            .collect::<Vec<_>>()
    };

    json!({
        "model": session.model,
        "instructions": format!("{}\n\n{}", session.system_prompt, build_agent_context_prompt(session, trigger)),
        "input": input,
        "text": {
            "format": {
                "type": "json_object"
            }
        }
    })
}

fn build_responses_compat_request_body(session: &AgentSession, trigger: Option<&str>) -> Value {
    let mut lines = vec![
        "System:".to_string(),
        session.system_prompt.clone(),
        String::new(),
        "Coordinator:".to_string(),
        build_agent_context_prompt(session, trigger),
    ];

    if session.messages.is_empty() {
        lines.push(String::new());
        lines.push("User: 请先用一句话说明你正在为星星的vibecoding启动器做初始化需求澄清，然后提出一个最关键的追问。".to_string());
    } else {
        for message in &session.messages {
            lines.push(String::new());
            lines.push(format!("{}: {}", message.role, message.content));
        }
    }

    json!({
        "model": session.model,
        "input": lines.join("\n"),
    })
}

fn post_agent_json(
    client: &Client,
    endpoint: &str,
    api_key: &str,
    request_body: &Value,
    call_log_context: Option<&AgentCallLogContext>,
) -> Result<Value, AgentHttpError> {
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(request_body)
        .send()
        .map_err(|error| {
            let error_text = error.to_string();
            append_agent_call_log(
                call_log_context,
                endpoint,
                request_body,
                None,
                None,
                None,
                None,
                None,
                Some(&error_text),
            );
            AgentHttpError {
                status_code: 0,
                body: error_text,
                endpoint: Some(endpoint.to_string()),
            }
        })?;

    let status = response.status();
    let status_code = status.as_u16();
    let success = status.is_success();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let body = match response.text() {
        Ok(body) => body,
        Err(error) => {
            let detail = error.to_string();
            append_agent_call_log(
                call_log_context,
                endpoint,
                request_body,
                Some(status_code),
                content_type.as_deref(),
                None,
                None,
                None,
                Some(&detail),
            );
            return Err(AgentHttpError {
                status_code,
                body: format!(
                    "response_body_read_failed reason={}",
                    sanitize_agent_error_detail(&detail)
                ),
                endpoint: Some(endpoint.to_string()),
            });
        }
    };
    if success {
        match parse_successful_agent_response_text(&body) {
            Ok(value) => {
                append_agent_call_log(
                    call_log_context,
                    endpoint,
                    request_body,
                    Some(status_code),
                    content_type.as_deref(),
                    Some(&body),
                    Some(&value),
                    None,
                    None,
                );
                return Ok(value);
            }
            Err(reason) => {
                append_agent_call_log(
                    call_log_context,
                    endpoint,
                    request_body,
                    Some(status_code),
                    content_type.as_deref(),
                    Some(&body),
                    None,
                    Some(&reason),
                    None,
                );
                return Err(AgentHttpError {
                    status_code,
                    body: format!(
                        "invalid_json reason={} bodySnippet={}",
                        reason,
                        sanitize_agent_error_detail(&body)
                    ),
                    endpoint: Some(endpoint.to_string()),
                });
            }
        }
    }

    let parsed_error_body = serde_json::from_str::<Value>(&body).ok();
    append_agent_call_log(
        call_log_context,
        endpoint,
        request_body,
        Some(status_code),
        content_type.as_deref(),
        Some(&body),
        parsed_error_body.as_ref(),
        None,
        None,
    );

    Err(AgentHttpError {
        status_code,
        body,
        endpoint: Some(endpoint.to_string()),
    })
}

fn map_agent_http_error(error: &AgentHttpError) -> String {
    if error.status_code == 401
        || error.status_code == 403
        || error.body.contains("INVALID_API_KEY")
        || error.body.contains("invalid_api_key")
    {
        return "agent_auth_failed".to_string();
    }
    if error.status_code == 400 {
        return "agent_request_bad_request".to_string();
    }
    if error.status_code == 200 && error.body.starts_with("invalid_json") {
        let endpoint = error.endpoint.as_deref().unwrap_or("unknown");
        return format!(
            "agent_response_invalid status=200 endpoint={} {}",
            endpoint,
            sanitize_agent_error_detail(&error.body)
        );
    }
    if error.status_code == 200 && error.body.starts_with("response_body_read_failed") {
        let endpoint = error.endpoint.as_deref().unwrap_or("unknown");
        return format!(
            "模型请求失败：模型服务已返回响应头，但正文读取失败或超时。endpoint={} {}",
            endpoint,
            sanitize_agent_error_detail(&error.body)
        );
    }
    if error.status_code == 0 {
        let detail = sanitize_agent_error_detail(&error.body);
        if detail.is_empty() {
            return "模型请求失败：无法连接到模型服务，请检查网络、代理、防火墙和 API Base URL。"
                .to_string();
        }
        return format!(
            "模型请求失败：无法连接到模型服务，请检查网络、代理、防火墙和 API Base URL。底层错误：{}",
            detail
        );
    }

    if let Some(endpoint) = error.endpoint.as_deref() {
        return format!("agent_request_failed_{}: {}", error.status_code, endpoint);
    }

    format!("agent_request_failed_{}", error.status_code)
}

fn sanitize_agent_error_detail(detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut redacted = trimmed.replace("Bearer ", "Bearer [redacted] ");
    while let Some(start) = redacted.find("api_key=") {
        let value_start = start + "api_key=".len();
        let value_end = redacted[value_start..]
            .find(char::is_whitespace)
            .map(|offset| value_start + offset)
            .unwrap_or(redacted.len());
        redacted.replace_range(value_start..value_end, "[redacted]");
    }
    let redacted = redacted.replace("api_key", "api_key[redacted]");
    redact_inline_api_key_tokens(&redacted)
        .chars()
        .take(500)
        .collect()
}

fn redact_inline_api_key_tokens(input: &str) -> String {
    let mut output = String::new();
    let mut index = 0usize;
    while index < input.len() {
        if input[index..].starts_with("sk-") {
            output.push_str("[redacted-api-key]");
            index += "sk-".len();
            while index < input.len() {
                let ch = input[index..]
                    .chars()
                    .next()
                    .expect("index should stay on a char boundary");
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    index += ch.len_utf8();
                } else {
                    break;
                }
            }
            continue;
        }

        let ch = input[index..]
            .chars()
            .next()
            .expect("index should stay on a char boundary");
        output.push(ch);
        index += ch.len_utf8();
    }
    output
}

fn parse_successful_agent_response_text(raw_body: &str) -> Result<Value, String> {
    let trimmed = raw_body.trim();
    if trimmed.is_empty() {
        return Err("empty_body".to_string());
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }

    if let Some(value) = parse_sse_agent_response_text(trimmed) {
        return Ok(value);
    }

    if let Some(candidate) = extract_json_object(trimmed) {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            return Ok(value);
        }
    }

    if trimmed.starts_with('<') {
        return Err("html_body".to_string());
    }

    Ok(Value::String(trimmed.to_string()))
}

fn parse_sse_agent_response_text(raw_body: &str) -> Option<Value> {
    for line in raw_body.lines() {
        let trimmed = line.trim();
        let Some(data) = trimmed.strip_prefix("data:") else {
            continue;
        };
        let payload = data.trim();
        if payload.is_empty() || payload == "[DONE]" {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(payload) {
            return Some(value);
        }
        if let Some(candidate) = extract_json_object(payload) {
            if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
                return Some(value);
            }
        }
    }

    None
}

fn should_fallback_from_responses_to_chat(error: &AgentHttpError) -> bool {
    matches!(
        error.status_code,
        400 | 404 | 405 | 408 | 409 | 415 | 422 | 429
    ) || error.status_code >= 500
}

fn parse_agent_decision_response(value: &Value) -> Result<AgentDecision, String> {
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    let mut last_error: Option<String> = None;

    for raw_content in candidates {
        match parse_agent_decision_content(&raw_content) {
            Ok(decision) => return Ok(decision),
            Err(error) if is_immediate_agent_contract_error(&error) => return Err(error),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| "agent_response_missing_content".to_string()))
}

fn parse_agent_decision_response_for_provider(
    value: &Value,
    provider: &str,
) -> Result<AgentDecision, String> {
    if !provider_allows_minimax_reasoning_cleanup(provider) {
        return parse_agent_decision_response(value);
    }

    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    let mut last_error: Option<String> = None;

    for raw_content in candidates {
        if let Some(cleaned) = strip_reasoning_think_blocks(&raw_content) {
            let cleaned = cleaned.trim();
            if !cleaned.is_empty() && cleaned != raw_content.trim() {
                match parse_agent_decision_content(cleaned) {
                    Ok(decision) => return Ok(decision),
                    Err(error) if is_immediate_agent_contract_error(&error) => return Err(error),
                    Err(error) => {
                        last_error = Some(error);
                        if looks_like_json_candidate(cleaned) {
                            continue;
                        }
                    }
                }
            } else {
                last_error = Some("agent_response_not_json".to_string());
            }
            continue;
        }

        match parse_agent_decision_content(&raw_content) {
            Ok(decision) => return Ok(decision),
            Err(error) if is_immediate_agent_contract_error(&error) => return Err(error),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| "agent_response_missing_content".to_string()))
}

fn provider_allows_minimax_reasoning_cleanup(provider: &str) -> bool {
    provider.trim().eq_ignore_ascii_case("custom")
}

fn should_retry_agent_parse_with_chat_fallback(error: &str) -> bool {
    matches!(
        error,
        "agent_response_missing_content" | "agent_response_not_json"
    )
}

fn collect_text_candidates(value: &Value, candidates: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                candidates.push(text.clone());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_text_candidates(item, candidates);
            }
        }
        Value::Object(map) => {
            for key in ["output_text", "content", "text", "value", "refusal"] {
                if let Some(value) = map.get(key) {
                    collect_text_candidates(value, candidates);
                }
            }

            for key in ["message", "output", "choices"] {
                if let Some(value) = map.get(key) {
                    collect_text_candidates(value, candidates);
                }
            }
        }
        _ => {}
    }
}

fn parse_agent_decision_content(raw_content: &str) -> Result<AgentDecision, String> {
    if looks_like_unstructured_solution_bundle(raw_content) {
        return Err("agent_solutions_unstructured_markdown".to_string());
    }

    let trimmed = raw_content.trim();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return Err("agent_response_not_json".to_string());
    }

    if is_raw_json_object_text(trimmed) {
        return parse_agent_decision_json(trimmed);
    }

    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Err("agent_response_not_json".to_string());
    }

    if let Ok(encoded) = serde_json::from_str::<String>(trimmed) {
        match parse_non_raw_agent_decision_candidate(&encoded) {
            Ok(decision) => return Ok(decision),
            Err(error) if is_immediate_agent_contract_error(&error) => return Err(error),
            Err(_) => {}
        }
        if looks_like_json_candidate(&encoded) {
            return Err("agent_response_not_json".to_string());
        }
    }

    match parse_non_raw_agent_decision_candidate(trimmed) {
        Ok(decision) => return Ok(decision),
        Err(error) if is_immediate_agent_contract_error(&error) => return Err(error),
        Err(_) => {}
    }

    if looks_like_json_candidate(trimmed) || trimmed.starts_with('"') {
        return Err("agent_response_not_json".to_string());
    }

    if let Some(fallback) = fallback_decision_from_plain_text(trimmed) {
        return Ok(fallback);
    }

    Err("agent_response_not_json".to_string())
}

fn parse_non_raw_agent_decision_candidate(raw_content: &str) -> Result<AgentDecision, String> {
    if looks_like_unstructured_solution_bundle(raw_content) {
        return Err("agent_solutions_unstructured_markdown".to_string());
    }

    let trimmed = raw_content.trim();
    if trimmed.is_empty() || trimmed.starts_with('<') {
        return Err("agent_response_not_json".to_string());
    }

    if is_raw_json_object_text(trimmed) {
        let decision = parse_agent_decision_json(trimmed)?;
        return reject_non_raw_solutions(decision);
    }

    if let Some(candidate) = extract_json_object(trimmed) {
        let decision = parse_agent_decision_json(&candidate)?;
        return reject_non_raw_solutions(decision);
    }

    Err("agent_response_not_json".to_string())
}

fn reject_non_raw_solutions(decision: AgentDecision) -> Result<AgentDecision, String> {
    if decision.mode == "solutions" {
        Err("agent_solutions_unstructured_markdown".to_string())
    } else {
        Ok(decision)
    }
}

fn is_raw_json_object_text(raw_content: &str) -> bool {
    let trimmed = raw_content.trim();
    trimmed.starts_with('{') && trimmed.ends_with('}')
}

fn strip_reasoning_think_blocks(raw_content: &str) -> Option<String> {
    strip_leading_xml_like_blocks(raw_content, "think")
}

fn strip_leading_xml_like_blocks(raw_content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{tag}>");
    let close_tag = format!("</{tag}>");
    let mut remaining = raw_content;
    let mut changed = false;

    loop {
        let trimmed = remaining.trim_start();
        let trimmed_lower = trimmed.to_ascii_lowercase();
        if !trimmed_lower.starts_with(&open_tag) {
            return changed.then(|| trimmed.to_string());
        }

        changed = true;
        let after_open_index = open_tag.len();
        let after_open_lower = &trimmed_lower[after_open_index..];
        let Some(close_relative_index) = after_open_lower.find(&close_tag) else {
            return Some(String::new());
        };
        let after_close_index = after_open_index + close_relative_index + close_tag.len();
        remaining = &trimmed[after_close_index..];
    }
}

fn parse_agent_decision_json(raw_content: &str) -> Result<AgentDecision, String> {
    let decision = serde_json::from_str::<AgentDecision>(raw_content)
        .map_err(|_| "agent_response_not_json".to_string())?;
    validate_agent_decision_contract(&decision)?;
    Ok(decision)
}

fn validate_agent_decision_contract(decision: &AgentDecision) -> Result<(), String> {
    if decision.mode == "solutions" {
        let solutions = decision
            .solutions
            .as_ref()
            .ok_or_else(|| "agent_solutions_missing".to_string())?;
        validate_solution_rationale(solutions)?;
    }

    Ok(())
}

fn validate_solution_rationale(solutions: &[AgentSolution]) -> Result<(), String> {
    for solution in solutions {
        if solution.role_rationale.is_empty() || solution.omitted_role_rationale.is_empty() {
            return Err("agent_solution_role_rationale_missing".to_string());
        }
    }

    Ok(())
}

fn fallback_decision_from_plain_text(raw_content: &str) -> Option<AgentDecision> {
    let trimmed = raw_content.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('<')
        || trimmed.starts_with('{')
        || trimmed.starts_with('[')
        || trimmed.starts_with('"')
        || trimmed.starts_with("```")
    {
        return None;
    }

    Some(AgentDecision {
        mode: "question".to_string(),
        assistant_message: trimmed.to_string(),
        understanding_summary: None,
        readiness: None,
        solutions: None,
    })
}

fn looks_like_unstructured_solution_bundle(content: &str) -> bool {
    let normalized = content.to_ascii_lowercase();
    let solution_marker_count = [
        "方案a",
        "方案 a",
        "方案A",
        "方案 A",
        "option a",
        "solution a",
        "方案b",
        "方案 b",
        "方案B",
        "方案 B",
        "option b",
        "solution b",
        "方案c",
        "方案 c",
        "方案C",
        "方案 C",
        "option c",
        "solution c",
    ]
    .iter()
    .filter(|marker| {
        content.contains(**marker) || normalized.contains(&marker.to_ascii_lowercase())
    })
    .count();
    let has_team_payload = content.contains("Agent团队组成")
        || content.contains("团队组成")
        || normalized.contains("teamcomposition")
        || normalized.contains("rolerationale")
        || normalized.contains("omittedrolerationale");
    let asks_chat_selection = content.contains("请选择")
        || content.contains("选择偏好")
        || content.contains("选中推荐方案")
        || content.contains("A/B/C")
        || normalized.contains("solution-picker-agent")
        || normalized.contains("choose");

    solution_marker_count >= 3 && has_team_payload && asks_chat_selection
}

fn extract_json_object(raw_content: &str) -> Option<String> {
    let start = raw_content.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in raw_content[start..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escaped = true;
            }
            '"' => {
                in_string = !in_string;
            }
            '{' if !in_string => {
                depth += 1;
            }
            '}' if !in_string => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start + offset;
                    return Some(raw_content[start..=end].to_string());
                }
            }
            _ => {}
        }
    }

    None
}

fn build_system_prompt(payload_root: &Path, workspace_path: &str) -> Result<String, String> {
    let product_manager = read_reference_file(
        payload_root,
        &["product-manager.md", "references/product-manager.md"],
    )?;
    let agent_catalog = read_reference_file(
        payload_root,
        &[
            "agency-agents-zh/README.md",
            "references/agency-agents-zh-README.md",
        ],
    )?;

    Ok(format!(
        "You are the main initialization agent for 星星的vibecoding启动器.\n\
You must follow the working style implied by the following product-manager reference:\n\
{}\n\
\n\
You must use the following agency-agent catalog reference as one input when deciding the team composition:\n\
{}\n\
\n\
Start behavior: 先理解用户想法，再根据已加载的 agency-agent catalog reference 判断需要加入团队的 agent。\n\
Stop behavior: 输出三个明确的解决方案，包含架构、agent团队组成、token预估、角色选择理由、明显候选角色不选理由，然后交给内置方案选择 UI。星梦梦会在成功前复核你的方案与最终产物。\n\
Do not behave like a fixed questionnaire. Ask natural follow-up questions. Current workspace: {}.",
        limit_prompt_size(&product_manager, 8000),
        limit_prompt_size(&agent_catalog, 12000),
        workspace_path
    ))
}

fn read_reference_file(payload_root: &Path, candidates: &[&str]) -> Result<String, String> {
    for candidate in candidates {
        let path = payload_root.join(candidate);
        if path.is_file() {
            return fs::read_to_string(path).map_err(|_| "reference_file_unreadable".to_string());
        }
    }

    Err("reference_file_missing".to_string())
}

fn limit_prompt_size(content: &str, max_chars: usize) -> String {
    if content.chars().count() <= max_chars {
        return content.to_string();
    }

    content.chars().take(max_chars).collect()
}

fn build_tool_calls(session: &AgentSession) -> Vec<AgentToolCall> {
    if !session.tool_calls.is_empty() {
        return session.tool_calls.clone();
    }

    if session.finished && session.selected_solution_id.is_some() {
        return vec![AgentToolCall {
            tool_name: "open_solution_selector".to_string(),
            status: "completed".to_string(),
            payload_json: session
                .selected_solution_id
                .as_ref()
                .map(|solution_id| json!({ "selectedSolutionId": solution_id }).to_string()),
        }];
    }

    if !session.finished && !session.solutions.is_empty() {
        return vec![AgentToolCall {
            tool_name: "open_solution_selector".to_string(),
            status: "requested".to_string(),
            payload_json: Some(json!({ "solutionCount": session.solutions.len() }).to_string()),
        }];
    }

    Vec::new()
}

fn normalize_assistant_message(message: &str, readiness: &AgentReadiness) -> String {
    let trimmed = message.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    build_readiness_follow_up(readiness)
}

fn merge_readiness(
    current: &AgentReadiness,
    incoming: Option<AgentReadiness>,
    messages: &[AgentMessage],
    summary_hint: Option<&str>,
    provider: &str,
) -> AgentReadiness {
    let mut readiness =
        incoming.unwrap_or_else(|| infer_readiness_from_messages(messages, summary_hint));
    let summary_presented = readiness.summary_presented
        || current.summary_presented
        || summary_hint
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        || assistant_has_presented_summary(messages);
    let latest_user = messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.as_str())
        .unwrap_or_default();
    let summary_confirmed = current.summary_confirmed
        || (summary_presented
            && is_explicit_confirmation_message_for_provider(provider, latest_user));

    if readiness.product_type.is_none() {
        readiness.product_type = current.product_type.clone();
    }
    if readiness.target_users.is_none() {
        readiness.target_users = current.target_users.clone();
    }
    if readiness.core_problem.is_none() {
        readiness.core_problem = current.core_problem.clone();
    }
    if readiness.key_features.is_empty() {
        readiness.key_features = current.key_features.clone();
    }
    if readiness.constraints.is_empty() {
        readiness.constraints = current.constraints.clone();
    }
    readiness.summary_presented = summary_presented;
    readiness.summary_confirmed = summary_confirmed;
    readiness.missing_fields = compute_missing_fields(&readiness);
    readiness.ready_for_solutions =
        readiness.missing_fields.is_empty() && readiness.summary_confirmed;
    readiness
}

fn readiness_has_required_details(readiness: &AgentReadiness) -> bool {
    readiness
        .product_type
        .as_deref()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && readiness
            .target_users
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        && readiness
            .core_problem
            .as_deref()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
        && !readiness.key_features.is_empty()
        && !readiness.constraints.is_empty()
}

fn infer_readiness_from_messages(
    messages: &[AgentMessage],
    summary_hint: Option<&str>,
) -> AgentReadiness {
    let user_messages = messages
        .iter()
        .filter(|message| message.role == "user")
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>();
    let combined = user_messages.join("\n");
    let latest_user = user_messages.last().copied().unwrap_or_default();
    let lower = combined.to_lowercase();

    let product_type = if lower.contains("mcp") {
        Some("MCP".to_string())
    } else if lower.contains("skill") {
        Some("Skill".to_string())
    } else if lower.contains("网站")
        || lower.contains("web")
        || lower.contains("商城")
        || lower.contains("shop")
    {
        Some("网站".to_string())
    } else if lower.contains("软件") || lower.contains("桌面") || lower.contains("app") {
        Some("软件".to_string())
    } else {
        None
    };

    let target_users = extract_segment(latest_user, &["目标用户", "用户", "面向", "给"]);
    let core_problem = extract_segment(latest_user, &["解决", "问题", "痛点", "帮助"]);
    let key_features = extract_list(latest_user, &["功能", "支持", "需要", "包括"]);
    let constraints = extract_list(
        latest_user,
        &["约束", "限制", "时间", "预算", "部署", "技术"],
    );
    let summary_presented = summary_hint
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        || assistant_has_presented_summary(messages);
    let summary_confirmed = summary_presented && is_explicit_confirmation_message(latest_user);

    let mut readiness = AgentReadiness {
        product_type,
        target_users,
        core_problem,
        key_features,
        constraints,
        summary_presented,
        summary_confirmed,
        missing_fields: Vec::new(),
        ready_for_solutions: false,
    };
    readiness.missing_fields = compute_missing_fields(&readiness);
    readiness.ready_for_solutions =
        readiness.missing_fields.is_empty() && readiness.summary_confirmed;
    readiness
}

fn extract_segment(text: &str, keywords: &[&str]) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    for keyword in keywords {
        if trimmed.contains(keyword) {
            return Some(trimmed.to_string());
        }
    }

    None
}

fn extract_list(text: &str, keywords: &[&str]) -> Vec<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    if !keywords.iter().any(|keyword| trimmed.contains(keyword)) {
        return Vec::new();
    }

    trimmed
        .split(['，', ',', '、', '\n'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn compute_missing_fields(readiness: &AgentReadiness) -> Vec<String> {
    let mut missing = Vec::new();

    if readiness.product_type.as_deref().unwrap_or("").is_empty() {
        missing.push("产品形态".to_string());
    }
    if readiness.target_users.as_deref().unwrap_or("").is_empty() {
        missing.push("目标用户".to_string());
    }
    if readiness.core_problem.as_deref().unwrap_or("").is_empty() {
        missing.push("核心问题".to_string());
    }
    if readiness.key_features.is_empty() {
        missing.push("关键功能".to_string());
    }
    if readiness.constraints.is_empty() {
        missing.push("约束条件".to_string());
    }
    if !readiness.summary_confirmed {
        missing.push("用户确认".to_string());
    }

    missing
}

fn assistant_has_presented_summary(messages: &[AgentMessage]) -> bool {
    messages.iter().any(|message| {
        message.role == "assistant"
            && (message.content.contains("这样理解对吗")
                || message.content.contains("如果准确，我就继续整理三套方案")
                || message.content.contains("目前理解为："))
    })
}

fn latest_user_message(messages: &[AgentMessage]) -> Option<&str> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.as_str())
}

fn is_explicit_confirmation_message_for_provider(provider: &str, message: &str) -> bool {
    is_explicit_confirmation_message(message)
        || (provider_allows_minimax_reasoning_cleanup(provider)
            && is_custom_compact_confirmation_message(message))
}

fn is_custom_compact_confirmation_message(message: &str) -> bool {
    let normalized = normalize_confirmation_text(message);
    normalized == "准确" || normalized == "确认准确"
}

fn is_explicit_confirmation_message(message: &str) -> bool {
    let normalized = normalize_confirmation_text(message);

    if normalized.is_empty() {
        return false;
    }

    let negative_markers = [
        "不是",
        "不对",
        "不准确",
        "不行",
        "不可以",
        "不确定",
        "不确认",
        "不能确定",
        "还不能确认",
        "还不对",
        "有问题",
    ];
    if negative_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return false;
    }

    let compact_positive_markers = ["准确", "确认准确", "确定", "确认", "没问题"];
    if compact_positive_markers
        .iter()
        .any(|marker| normalized == *marker)
    {
        return true;
    }

    let positive_markers = [
        "是的",
        "没错",
        "对的",
        "正确",
        "就这样",
        "可以",
        "可以的",
        "按这个理解",
        "总结准确",
        "理解正确",
        "这个总结准确",
    ];

    positive_markers
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn normalize_confirmation_text(message: &str) -> String {
    message
        .trim()
        .replace(['。', '，', ',', '！', '!', '？', '?', ' '], "")
        .to_lowercase()
}

fn build_readiness_summary(readiness: &AgentReadiness) -> String {
    format!(
        "目前理解为：产品形态是{}；目标用户是{}；核心问题是{}；关键功能包括{}；约束条件是{}。",
        readiness.product_type.as_deref().unwrap_or("待确认"),
        readiness.target_users.as_deref().unwrap_or("待确认"),
        readiness.core_problem.as_deref().unwrap_or("待确认"),
        if readiness.key_features.is_empty() {
            "待确认".to_string()
        } else {
            readiness.key_features.join("、")
        },
        if readiness.constraints.is_empty() {
            "待确认".to_string()
        } else {
            readiness.constraints.join("、")
        }
    )
}

fn build_readiness_follow_up(readiness: &AgentReadiness) -> String {
    let detail_missing = readiness
        .missing_fields
        .iter()
        .filter(|field| field.as_str() != "用户确认")
        .cloned()
        .collect::<Vec<_>>();

    if detail_missing.is_empty() && !readiness.summary_confirmed {
        return format!(
            "{} 这样理解对吗？如果准确，我就继续整理三套方案。",
            build_readiness_summary(readiness)
        );
    }

    let missing = if detail_missing.is_empty() {
        "你的目标细节".to_string()
    } else {
        detail_missing.join("、")
    };

    format!(
        "为了继续整理方案，星星还需要你补充或确认这些信息：{}。",
        missing
    )
}

pub(crate) fn execute_solution_bootstrap(
    session: &AgentSession,
) -> Result<AgentBootstrapResult, String> {
    let runtime = ProviderSemanticAgentRuntime;
    execute_solution_bootstrap_with_runtime(session, &runtime)
}

fn execute_solution_bootstrap_with_runtime(
    session: &AgentSession,
    runtime: &dyn SemanticAgentRuntime,
) -> Result<AgentBootstrapResult, String> {
    let mut semantic_session = session.clone();
    let session_root = prepare_bootstrap_session(&semantic_session)?;

    let pre_bootstrap_context = SemanticReviewContext::pre_bootstrap(&semantic_session);
    let pre_bootstrap_acceptance = run_semantic_acceptance_gate_with_runtime(
        &mut semantic_session,
        &session_root,
        runtime,
        &pre_bootstrap_context,
    )?;
    if !pre_bootstrap_acceptance.passed {
        return Ok(AgentBootstrapResult {
            status: "failure".to_string(),
            workspace_path: semantic_session.workspace_path.clone(),
            generated_files: Vec::new(),
            handoff_path: None,
            postcheck_passed: false,
            user_facing_message: format!(
                "星梦梦语义验收未通过：{}",
                pre_bootstrap_acceptance.blocking_issues.join("；")
            ),
        });
    }

    let session_root = prepare_bootstrap_session(&semantic_session)?;
    let session_root_text = session_root.to_string_lossy().to_string();

    shell::run_orchestrator(
        &semantic_session.orchestrator_path,
        &OrchestratorRequest {
            stage: "confirm".to_string(),
            session_root: Some(session_root_text.clone()),
            project_root: None,
            input_text: None,
            choice: semantic_session.selected_solution_id.clone(),
            target_root: None,
            values_path: None,
            provider: None,
            model: None,
            api_key: None,
            base_url: None,
            execute: false,
            force: false,
        },
        Some("powershell"),
    )?;

    let bootstrap_output = shell::run_orchestrator(
        &semantic_session.orchestrator_path,
        &OrchestratorRequest {
            stage: "bootstrap".to_string(),
            session_root: Some(session_root_text),
            project_root: None,
            input_text: None,
            choice: None,
            target_root: Some(semantic_session.workspace_path.clone()),
            values_path: None,
            provider: Some(semantic_session.provider.clone()),
            model: Some(semantic_session.model.clone()),
            api_key: Some(semantic_session.api_key.clone()),
            base_url: Some(semantic_session.base_url.clone()),
            execute: true,
            force: true,
        },
        Some("powershell"),
    )?;

    let bootstrap_result =
        parse_bootstrap_result(&bootstrap_output, &semantic_session.workspace_path)?;
    if bootstrap_result.status != "success" {
        return Ok(bootstrap_result);
    }

    refresh_session_capabilities_from_generated_decision(&mut semantic_session);
    let final_context = SemanticReviewContext::final_package(&semantic_session, &bootstrap_result);
    let final_acceptance = run_semantic_acceptance_gate_with_runtime(
        &mut semantic_session,
        &session_root,
        runtime,
        &final_context,
    )?;
    if !final_acceptance.passed {
        return Ok(AgentBootstrapResult {
            status: "failure".to_string(),
            workspace_path: semantic_session.workspace_path.clone(),
            generated_files: bootstrap_result.generated_files,
            handoff_path: bootstrap_result.handoff_path,
            postcheck_passed: false,
            user_facing_message: format!(
                "星梦梦最终语义验收未通过：{}",
                final_acceptance.blocking_issues.join("；")
            ),
        });
    }

    Ok(bootstrap_result)
}

fn refresh_session_capabilities_from_generated_decision(session: &mut AgentSession) {
    let decision_path = PathBuf::from(&session.workspace_path)
        .join(".commonhe")
        .join("session")
        .join("decision.json");
    let Some(capabilities) = fs::read_to_string(&decision_path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .and_then(|value| value.get("selected_capabilities").cloned())
        .and_then(|value| serde_json::from_value::<Vec<SelectedCapability>>(value).ok())
    else {
        return;
    };

    let normalized = resolve_selected_capabilities(capabilities);
    if !normalized.is_empty() {
        session.selected_capabilities = normalized;
    }
}

fn run_semantic_acceptance_gate_with_runtime(
    session: &mut AgentSession,
    session_root: &Path,
    runtime: &dyn SemanticAgentRuntime,
    context: &SemanticReviewContext,
) -> Result<SemanticReviewResult, String> {
    const MAX_SEMANTIC_REVIEW_ROUNDS: usize = 5;

    let mut dialogue_rounds = Vec::new();
    let mut final_review = None;

    for round in 1..=MAX_SEMANTIC_REVIEW_ROUNDS {
        session.semantic_review_status = "reviewing".to_string();
        let review = sanitize_semantic_review_against_session(
            session,
            merge_semantic_safety_floor(session, runtime.review(session, round, context)?),
            context,
        );
        session.semantic_review_issues = review.blocking_issues.clone();
        session.dialogue_round_count = round;

        if review.passed {
            session.semantic_review_status = "passed".to_string();
            session.semantic_review_issues.clear();
            dialogue_rounds.push(AgentDialogueRound {
                round,
                reviewer_agent: "星梦梦".to_string(),
                main_agent: "梦星星".to_string(),
                review: review.clone(),
                repair: None,
            });
            final_review = Some(review);
            break;
        }

        if round == MAX_SEMANTIC_REVIEW_ROUNDS {
            session.semantic_review_status = "failed".to_string();
            dialogue_rounds.push(AgentDialogueRound {
                round,
                reviewer_agent: "星梦梦".to_string(),
                main_agent: "梦星星".to_string(),
                review: review.clone(),
                repair: None,
            });
            final_review = Some(review);
            break;
        }

        session.semantic_review_status = "repairing".to_string();
        let repair = runtime.repair(session, &review, round)?;
        if let Some(updated_solution) = repair.updated_solution.clone() {
            apply_repaired_solution(session, updated_solution)?;
        }
        dialogue_rounds.push(AgentDialogueRound {
            round,
            reviewer_agent: "星梦梦".to_string(),
            main_agent: "梦星星".to_string(),
            review,
            repair: Some(repair),
        });
    }

    let review = final_review.ok_or_else(|| "semantic_review_missing_final_result".to_string())?;
    write_semantic_review_artifacts(session, session_root, &review, &dialogue_rounds)?;
    Ok(review)
}

trait SemanticAgentRuntime {
    fn review(
        &self,
        session: &AgentSession,
        round: usize,
        context: &SemanticReviewContext,
    ) -> Result<SemanticReviewResult, String>;
    fn repair(
        &self,
        session: &AgentSession,
        review: &SemanticReviewResult,
        round: usize,
    ) -> Result<RepairDecision, String>;
}

struct ProviderSemanticAgentRuntime;

impl SemanticAgentRuntime for ProviderSemanticAgentRuntime {
    fn review(
        &self,
        session: &AgentSession,
        round: usize,
        context: &SemanticReviewContext,
    ) -> Result<SemanticReviewResult, String> {
        request_semantic_review(session, round, context)
    }

    fn repair(
        &self,
        session: &AgentSession,
        review: &SemanticReviewResult,
        round: usize,
    ) -> Result<RepairDecision, String> {
        request_meng_xingxing_repair(session, review, round)
    }
}

fn apply_repaired_solution(
    session: &mut AgentSession,
    mut updated_solution: AgentSolution,
) -> Result<(), String> {
    let selected_solution_id = session
        .selected_solution_id
        .clone()
        .ok_or_else(|| "semantic_repair_missing_selected_solution".to_string())?;
    if updated_solution.id != selected_solution_id {
        return Err("semantic_repair_solution_id_mismatch".to_string());
    }

    if let Some(solution) = session
        .solutions
        .iter_mut()
        .find(|solution| solution.id == selected_solution_id)
    {
        normalize_repaired_solution_rationale(&mut updated_solution);
        *solution = updated_solution;
        Ok(())
    } else {
        Err("semantic_repair_selected_solution_not_found".to_string())
    }
}

fn normalize_repaired_solution_rationale(solution: &mut AgentSolution) {
    normalize_business_architecture_summary(solution);

    let selected_roles = solution
        .team_composition
        .iter()
        .chain(solution.role_rationale.keys())
        .map(|role| normalize_role_key(role))
        .collect::<Vec<_>>();

    solution.omitted_role_rationale.retain(|role, _| {
        let omitted_role = normalize_role_key(role);
        !selected_roles
            .iter()
            .any(|selected_role| selected_role == &omitted_role)
    });

    for (role, rationale) in solution.omitted_role_rationale.iter_mut() {
        if canonical_role_from_label(role) == "product-manager"
            && (rationale.contains("梦星星与真源文档承接")
                || !rationale.to_ascii_lowercase().contains("architect")
                || !rationale.contains("产品决策"))
        {
            *rationale = default_omitted_package_role_rationale(role);
        } else if omitted_role_rationale_confuses_docs_with_business_content(role, rationale) {
            *rationale = default_omitted_package_role_rationale(role);
        }
    }

    normalize_docs_role_rationale(solution);

    let has_miniapp_role = selected_roles.iter().any(|role| role == "miniapp");
    if has_miniapp_role {
        if let Some(frontend_rationale) = solution.role_rationale.get_mut("frontend") {
            if frontend_rationale.contains("小程序")
                || frontend_rationale.to_ascii_lowercase().contains("miniapp")
                || frontend_rationale.to_ascii_lowercase().contains("uni-app")
            {
                *frontend_rationale = "负责 Web 端页面、交互和用户可见路径实现。".to_string();
            }
        }
    }

    normalize_low_code_solution_roles(solution);
    propagate_omitted_role_assignments_to_owner_roles(solution);
    ensure_product_manager_omission_alignment(solution);
    ensure_reviewer_qa_scope_separation(solution);
    ensure_ai_engineer_omission_alignment(solution);
    ensure_security_omission_alignment(solution);
    clarify_pending_mobile_delivery_assumption(solution);

    if token_estimate_needs_package_budget_normalization(&solution.token_estimate) {
        solution.token_estimate = default_package_token_estimate();
    } else if token_estimate_understates_complex_package(solution) {
        solution.token_estimate = default_complex_package_token_estimate(solution);
    }
}

fn normalize_docs_role_rationale(solution: &mut AgentSolution) {
    let Some(current) = solution.role_rationale.get("docs").cloned() else {
        return;
    };

    if docs_rationale_has_valid_codex_plan(&current) {
        return;
    }

    let rationale = default_package_role_rationale("docs", solution);
    solution
        .role_rationale
        .insert("docs".to_string(), rationale);
}

fn docs_rationale_has_valid_codex_plan(rationale: &str) -> bool {
    let mentions_codex_entries = rationale.contains("AGENTS.md") && rationale.contains(".codex");
    let marks_as_planned = rationale.contains("计划")
        || rationale.contains("将在")
        || rationale.to_ascii_lowercase().contains("bootstrap");
    let distinguishes_phase = rationale.contains("当前")
        && (rationale.contains("后续")
            || rationale.contains("将在")
            || rationale.contains("计划")
            || rationale.to_ascii_lowercase().contains("bootstrap"));
    let avoids_docs_as_generator = !rationale.contains("负责在 bootstrap 阶段生成")
        && !rationale.contains("将在 bootstrap 阶段为目标软件")
        && !rationale.contains("负责生成目标软件");
    let falsely_claims_existing = rationale.contains("已生成")
        || rationale.contains("已存在")
        || rationale.contains("已经生成")
        || rationale.contains("已经存在");

    mentions_codex_entries
        && marks_as_planned
        && distinguishes_phase
        && avoids_docs_as_generator
        && !falsely_claims_existing
}

fn ensure_product_manager_omission_alignment(solution: &mut AgentSolution) {
    let Some((_, product_rationale)) = solution
        .omitted_role_rationale
        .iter()
        .find(|(role, _)| canonical_role_from_label(role) == "product-manager")
    else {
        return;
    };

    let mentions_architect = product_rationale.to_ascii_lowercase().contains("architect")
        || product_rationale.contains("架构");
    let mentions_reviewer = product_rationale.to_ascii_lowercase().contains("reviewer")
        || product_rationale.contains("复核")
        || product_rationale.contains("范围漂移");

    if mentions_architect && solution_role_is_selected(solution, "architect") {
        let default_rationale = default_package_role_rationale("architect", solution);
        let architect_rationale = solution
            .role_rationale
            .entry("architect".to_string())
            .or_insert(default_rationale);
        strip_generic_product_manager_assignment(architect_rationale);
        if !architect_rationale.contains("需求梳理")
            || !architect_rationale.contains("用户故事优先级")
            || !architect_rationale.contains("MVP")
            || !architect_rationale.contains("范围")
        {
            append_rationale_clause(
                architect_rationale,
                "承接产品经理职责，负责需求梳理、用户故事优先级、MVP 范围把控、企业询价流程取舍和产品决策闭环",
            );
        }
    }

    if mentions_reviewer && solution_role_is_selected(solution, "reviewer") {
        let default_rationale = default_package_role_rationale("reviewer", solution);
        let reviewer_rationale = solution
            .role_rationale
            .entry("reviewer".to_string())
            .or_insert(default_rationale);
        if !reviewer_rationale.contains("范围漂移") {
            append_rationale_clause(
                reviewer_rationale,
                "在产品经理不单独生成时复核需求范围漂移和方案取舍一致性",
            );
        }
    }

    if solution_role_is_selected(solution, "qa") {
        let default_rationale = default_package_role_rationale("qa", solution);
        solution
            .role_rationale
            .entry("qa".to_string())
            .or_insert(default_rationale);
    }
}

fn ensure_reviewer_qa_scope_separation(solution: &mut AgentSolution) {
    if solution_role_is_selected(solution, "reviewer") {
        let default_reviewer_rationale = default_package_role_rationale("reviewer", solution);
        let reviewer_rationale = solution
            .role_rationale
            .entry("reviewer".to_string())
            .or_insert(default_reviewer_rationale.clone());
        let lower = reviewer_rationale.to_ascii_lowercase();
        if !reviewer_rationale.contains("语义复核")
            || !reviewer_rationale.contains("范围漂移")
            || !reviewer_rationale.contains("目标软件协作包")
            || !reviewer_rationale.contains("星梦梦")
            || reviewer_rationale.contains("技术回归")
            || lower.contains("qa")
        {
            *reviewer_rationale = default_reviewer_rationale;
        }
    }

    if solution_role_is_selected(solution, "qa") {
        let default_qa_rationale = default_package_role_rationale("qa", solution);
        let qa_rationale = solution
            .role_rationale
            .entry("qa".to_string())
            .or_insert(default_qa_rationale.clone());
        if !qa_rationale.contains("技术回归")
            || !qa_rationale.contains("关键路径")
            || qa_rationale.contains("角色取舍")
            || qa_rationale.contains("方案完整性")
        {
            *qa_rationale = default_qa_rationale;
        }
    }
}

fn ensure_ai_engineer_omission_alignment(solution: &mut AgentSolution) {
    let Some(ai_role_key) = solution
        .omitted_role_rationale
        .keys()
        .find(|role| role_is_ai_engineer(role))
        .cloned()
    else {
        return;
    };

    let ai_context = format!(
        "{}\n{}\n{}",
        solution.title, solution.architecture_summary, solution.recommendation_text
    );
    if !(ai_context.contains("AI")
        || ai_context.contains("知识库")
        || ai_context.contains("RAG")
        || ai_context.contains("模型")
        || ai_context.contains("导购"))
    {
        return;
    }

    if solution_role_is_selected(solution, "backend") {
        let default_backend_rationale = default_package_role_rationale("backend", solution);
        let backend_rationale = solution
            .role_rationale
            .entry("backend".to_string())
            .or_insert(default_backend_rationale);
        if !backend_rationale.contains("RAG") && !backend_rationale.contains("知识库搭建") {
            backend_rationale.push_str(
                "；承接 AI API 集成、RAG/知识库搭建边界、提示词/模型参数配置和效果验证。",
            );
        } else if !backend_rationale.contains("模型参数") && !backend_rationale.contains("提示词")
        {
            backend_rationale.push_str("；明确提示词/模型参数配置和效果验证边界。");
        }
    }

    solution.omitted_role_rationale.insert(
        ai_role_key,
        "独立 AI 工程师暂不生成；backend 承担 AI API 集成、RAG/知识库搭建边界、提示词/模型参数配置和效果验证，后续复杂模型调优再拆分独立 AI 角色。"
            .to_string(),
    );
}

fn role_is_ai_engineer(role: &str) -> bool {
    let lower = role.to_ascii_lowercase();
    role.contains("AI工程师")
        || role.contains("AI 工程师")
        || lower.contains("ai-engineer")
        || lower.contains("ai engineer")
        || (role.contains("人工智能") && role.contains("工程师"))
}

fn ensure_security_omission_alignment(solution: &mut AgentSolution) {
    let Some((security_role_key, security_rationale)) = solution
        .omitted_role_rationale
        .iter()
        .find(|(role, _)| canonical_role_from_label(role) == "compliance")
        .map(|(role, rationale)| (role.clone(), rationale.clone()))
    else {
        return;
    };

    let needs_assessment = security_rationale.contains("合规")
        || security_rationale.contains("安全")
        || security_rationale.contains("评估");
    if !needs_assessment {
        return;
    }

    if solution_role_is_selected(solution, "compliance") {
        let default_rationale = default_package_role_rationale("compliance", solution);
        let compliance_rationale = solution
            .role_rationale
            .entry("compliance".to_string())
            .or_insert(default_rationale);
        if !compliance_rationale.contains("安全合规") {
            compliance_rationale.push_str("；负责安全合规评估、权限边界和敏感数据处理。");
        }
        return;
    }

    if solution_role_is_selected(solution, "architect")
        || solution_role_is_selected(solution, "backend")
    {
        solution.omitted_role_rationale.insert(
            security_role_key,
            "独立安全工程师暂不生成；architect 负责安全合规评估、权限边界和平台安全假设校验，backend 负责认证授权、输入校验和安全实现，后续高风险阶段再拆分独立安全角色。"
                .to_string(),
        );
    }

    if solution_role_is_selected(solution, "architect") {
        let default_rationale = default_package_role_rationale("architect", solution);
        let architect_rationale = solution
            .role_rationale
            .entry("architect".to_string())
            .or_insert(default_rationale);
        if !architect_rationale.contains("安全合规") {
            architect_rationale.push_str("；承担安全合规性评估和权限边界设计职责。");
        }
    }

    if solution_role_is_selected(solution, "backend") {
        let default_rationale = default_package_role_rationale("backend", solution);
        let backend_rationale = solution
            .role_rationale
            .entry("backend".to_string())
            .or_insert(default_rationale);
        if !backend_rationale.contains("安全实现") {
            backend_rationale.push_str("；承担认证授权、输入校验和安全实现职责。");
        }
    }
}

fn solution_role_is_selected(solution: &AgentSolution, canonical_role: &str) -> bool {
    solution
        .team_composition
        .iter()
        .any(|role| role_label_matches_canonical(role, canonical_role))
        || solution
            .role_rationale
            .keys()
            .any(|role| role_label_matches_canonical(role, canonical_role))
}

fn normalize_low_code_solution_roles(solution: &mut AgentSolution) {
    if !solution_uses_low_code_platform(solution) {
        return;
    }

    if solution.role_rationale.contains_key("architect") {
        solution.role_rationale.insert(
            "architect".to_string(),
            "负责低代码平台选型、平台边界、扩展点、权限/数据边界和后续退出方案，把梦星星方案沉淀为可执行实施边界。"
                .to_string(),
        );
    }
    if solution.role_rationale.contains_key("frontend") {
        solution.role_rationale.insert(
            "frontend".to_string(),
            "负责低代码平台管理后台页面配置、组件定制、表单/报表可见路径和 UI 一致性；不承接微信小程序端。"
                .to_string(),
        );
    }
    if solution.role_rationale.contains_key("backend") {
        solution.role_rationale.insert(
            "backend".to_string(),
            "负责低代码平台 API 集成、权限/业务规则配置、外部服务适配和必要的定制扩展边界。"
                .to_string(),
        );
    }
    if solution.role_rationale.contains_key("database") {
        solution.role_rationale.insert(
            "database".to_string(),
            "负责低代码平台数据模型、字段关系、状态一致性、迁移策略、报表查询和关键统计边界。"
                .to_string(),
        );
    }
    if solution.role_rationale.contains_key("qa") {
        solution.role_rationale.insert(
            "qa".to_string(),
            "负责低代码配置流、权限、表单、报表、小程序查询入口和跨端一致性的验收与回归证据。"
                .to_string(),
        );
    }
    if solution.role_rationale.contains_key("miniapp") {
        solution.role_rationale.insert(
            "miniapp".to_string(),
            "负责微信小程序端学生/家长查询入口、端侧状态、跨端交互和与低代码平台 API 的适配。"
                .to_string(),
        );
    }

    for (role, rationale) in solution.omitted_role_rationale.iter_mut() {
        if canonical_role_from_label(role) == "mobile-developer"
            || role.contains("移动应用")
            || role.contains("小程序")
        {
            *rationale =
                "原生移动应用职责不生成独立 Agent；微信小程序端由 miniapp 角色承接，frontend 只负责低代码平台管理后台。"
                    .to_string();
        } else if role.contains("UI") || role.contains("设计师") {
            *rationale =
                "UI/UX 设计职责由 frontend 与 miniapp 分担：frontend 负责 Web/后台视觉一致性和组件配置，miniapp 负责小程序端页面与端侧交互，architect 把控体验优先级与范围取舍。"
                    .to_string();
        }
    }
}

fn solution_uses_low_code_platform(solution: &AgentSolution) -> bool {
    let text = format!(
        "{}\n{}\n{}",
        solution.title, solution.architecture_summary, solution.recommendation_text
    )
    .to_ascii_lowercase();
    text.contains("低代码")
        || text.contains("mendix")
        || text.contains("简道云")
        || text.contains("明道云")
        || text.contains("宜搭")
        || text.contains("low-code")
}

fn clarify_pending_mobile_delivery_assumption(solution: &mut AgentSolution) {
    let has_pending_mobile_shape = solution.omitted_role_rationale.values().any(|rationale| {
        rationale.contains("移动端形式待确认")
            || rationale.contains("移动端形态待确认")
            || (rationale.contains("移动端") && rationale.contains("待确认"))
    });
    if !has_pending_mobile_shape {
        return;
    }

    let architecture_clarification =
        "移动端形式待确认；当前移动端实现按已选方案中的移动端形态假设推进，最终可按用户确认调整。";
    if !solution.architecture_summary.contains("移动端形式待确认") {
        if !solution.architecture_summary.trim().is_empty()
            && !solution
                .architecture_summary
                .trim_end()
                .ends_with(['。', '；', ';'])
        {
            solution.architecture_summary.push('。');
        }
        solution
            .architecture_summary
            .push_str(architecture_clarification);
    }

    let rationale_clarification =
        "移动端形式待确认；当前按已选方案的移动端形态假设推进，最终可按用户确认调整。";
    for role in ["miniapp", "frontend", "architect"] {
        if let Some(rationale) = solution.role_rationale.get_mut(role) {
            if !rationale.contains("移动端形式待确认") {
                rationale.push_str("；");
                rationale.push_str(rationale_clarification);
            }
            break;
        }
    }
}

fn propagate_omitted_role_assignments_to_owner_roles(solution: &mut AgentSolution) {
    let omitted_rationales = solution
        .omitted_role_rationale
        .iter()
        .map(|(role, rationale)| (role.clone(), rationale.clone()))
        .collect::<Vec<_>>();
    for (omitted_role, rationale) in omitted_rationales {
        for owner_role in [
            "frontend",
            "miniapp",
            "backend",
            "architect",
            "docs",
            "qa",
            "devops",
        ] {
            if !solution
                .team_composition
                .iter()
                .any(|role| role_label_matches_canonical(role, owner_role))
                && !solution.role_rationale.contains_key(owner_role)
            {
                continue;
            }
            if !omitted_rationale_assigns_to_role(&rationale, owner_role) {
                continue;
            }
            let default_rationale = default_package_role_rationale(owner_role, solution);
            let owner_rationale = solution
                .role_rationale
                .entry(owner_role.to_string())
                .or_insert(default_rationale);
            let addition = omitted_assignment_summary(&omitted_role, &rationale);
            if !owner_rationale.contains(&addition)
                && !omitted_assignment_already_covered(owner_rationale, &omitted_role, &rationale)
            {
                append_rationale_clause(owner_rationale, &addition);
            }
        }
    }
}

fn omitted_rationale_assigns_to_role(rationale: &str, owner_role: &str) -> bool {
    let lower = rationale.to_ascii_lowercase();
    match owner_role {
        "frontend" => lower.contains("frontend") || rationale.contains("前端"),
        "miniapp" => {
            lower.contains("miniapp")
                || rationale.contains("小程序")
                || rationale.contains("移动端")
        }
        "backend" => lower.contains("backend") || rationale.contains("后端"),
        "architect" => lower.contains("architect") || rationale.contains("架构"),
        "docs" => lower.contains("docs") || rationale.contains("文档"),
        "qa" => lower.contains("qa") || rationale.contains("测试") || rationale.contains("质量"),
        "devops" => {
            lower.contains("devops") || rationale.contains("部署") || rationale.contains("运维")
        }
        _ => false,
    }
}

fn omitted_assignment_summary(omitted_role: &str, rationale: &str) -> String {
    let lower_role = omitted_role.to_ascii_lowercase();
    if lower_role.contains("rapid-prototyper") || rationale.contains("快速原型") {
        "承接快速原型制作职责".to_string()
    } else if lower_role.contains("dingtalk") || rationale.contains("钉钉") {
        "承接钉钉集成开发职责".to_string()
    } else if lower_role.contains("feishu") || rationale.contains("飞书") {
        "承接飞书集成开发职责".to_string()
    } else if lower_role.contains("security") || rationale.contains("安全") {
        "承接安全工程职责".to_string()
    } else if lower_role.contains("ui-designer")
        || omitted_role.contains("UI")
        || omitted_role.contains("UX")
        || rationale.contains("UI/UX")
    {
        "承接 UI/UX 设计职责".to_string()
    } else if canonical_role_from_label(omitted_role) == "product-manager" {
        "承接产品经理职责，负责需求梳理、用户故事优先级、MVP 范围把控和产品决策".to_string()
    } else {
        format!("承接 {} 的并入职责", omitted_role)
    }
}

fn omitted_assignment_already_covered(
    owner_rationale: &str,
    omitted_role: &str,
    rationale: &str,
) -> bool {
    let addition = omitted_assignment_summary(omitted_role, rationale);
    addition
        .split_whitespace()
        .any(|token| !token.is_empty() && owner_rationale.contains(token))
        || (rationale.contains("钉钉") && owner_rationale.contains("钉钉"))
        || (rationale.contains("飞书") && owner_rationale.contains("飞书"))
        || (rationale.contains("快速原型") && owner_rationale.contains("快速原型"))
        || (canonical_role_from_label(omitted_role) == "product-manager"
            && owner_rationale.contains("产品经理"))
}

fn append_rationale_clause(rationale: &mut String, addition: &str) {
    let trimmed = rationale
        .trim()
        .trim_end_matches(['。', '；', ';'])
        .trim()
        .to_string();
    *rationale = trimmed;
    let addition = addition
        .trim()
        .trim_start_matches(['。', '；', ';'])
        .trim_end_matches(['。', '；', ';'])
        .trim();
    if addition.is_empty() {
        return;
    }
    if !rationale.is_empty() {
        rationale.push('；');
    }
    rationale.push_str(addition);
    rationale.push('。');
}

fn strip_generic_product_manager_assignment(rationale: &mut String) {
    for pattern in [
        "；承接 产品经理 的并入职责",
        ";承接 产品经理 的并入职责",
        "；承接产品经理的并入职责",
        ";承接产品经理的并入职责",
        "承接 产品经理 的并入职责",
        "承接产品经理的并入职责",
    ] {
        *rationale = rationale.replace(pattern, "");
    }
    *rationale = rationale
        .replace("。；", "；")
        .replace("；。", "。")
        .trim()
        .to_string();
}

fn normalize_business_architecture_summary(solution: &mut AgentSolution) {
    for pattern in [
        "产品主名称：星星的vibecoding启动器。",
        "产品主名称: 星星的vibecoding启动器。",
        "产品主名称：星星的vibecoding启动器；",
        "产品主名称: 星星的vibecoding启动器;",
        "产品主名称：星星的vibecoding启动器",
        "产品主名称: 星星的vibecoding启动器",
    ] {
        solution.architecture_summary = solution.architecture_summary.replace(pattern, "");
    }
    solution.architecture_summary = solution
        .architecture_summary
        .trim_start_matches(['。', '；', ';', ' ', '\n', '\r', '\t'])
        .trim()
        .to_string();
}

fn omitted_role_rationale_confuses_docs_with_business_content(role: &str, rationale: &str) -> bool {
    let canonical = canonical_role_from_label(role);
    canonical == "marketing-content-creator"
        && (rationale.contains("docs")
            || rationale.contains("文档")
            || rationale.contains("内容创建")
            || rationale.contains("内容职责"))
}

fn token_estimate_needs_package_budget_normalization(token_estimate: &str) -> bool {
    let lower = token_estimate.to_lowercase();
    token_estimate.trim().is_empty()
        || token_estimate.contains("开发阶段")
        || token_estimate.contains("整体约")
        || !token_estimate.contains("不包含")
        || !(lower.contains("token") || token_estimate.contains("预算"))
}

fn token_estimate_understates_complex_package(solution: &AgentSolution) -> bool {
    let role_count = solution
        .team_composition
        .len()
        .max(solution.role_rationale.len());
    role_count >= 8
        && (solution.token_estimate.contains("10 万")
            || solution.token_estimate.contains("10万")
            || solution.token_estimate.contains("100K")
            || solution.token_estimate.contains("100k"))
}

fn normalize_role_key(value: &str) -> String {
    value.trim().to_lowercase()
}

fn request_semantic_review(
    session: &AgentSession,
    round: usize,
    context: &SemanticReviewContext,
) -> Result<SemanticReviewResult, String> {
    let prompt = "你是星梦梦，星星的vibecoding启动器内部的极端挑刺语义验收 Agent。你必须检查用户原始需求、梦星星输出、已选方案、目标软件、能力状态和真源规则。审查重点是 selectedSolution 和最终生成的初始化协作包；未选方案只在破坏“三方案完整性”或明显误导用户选择时才作为阻断项。reviewPhase=pre_bootstrap_solution_review 时文件尚未生成，generatedFiles/generatedFileEvidence 为空是预期状态，不能因此阻断；targetClient 入口文件只能在 final_generated_package_review 阶段要求真实证据。pre_bootstrap 阶段允许 selectedSolution.roleRationale 以计划/交接契约形式提到后续会生成的 targetClient 入口路径，例如 codex 的 AGENTS.md/.codex；只有当梦星星声称这些文件已经存在、已经通过 postcheck，或把入口路径当成业务运行环境时才阻断。产品主名称规则用于启动器自身对外名称，不能要求把用户项目名或协作包名替换成“星星的vibecoding启动器”。capabilityState 中 fallback 表示桌面端已把能力记录进协作包但当前未做外部强校验，不能仅因 fallback 阻断，除非能力未选择、缺失或与方案硬矛盾。selectedCapabilities 是启动器/协作包工作流能力清单，不是业务应用依赖清单；不要因为认证、支付、存储、部署、数据库、Supabase、AI 平台、知识库平台或第三方 SDK 没有出现在 selectedCapabilities 中而阻断。targetClient 只表示后续接管软件和入口文件，例如 codex 生成 AGENTS.md/.codex；它不是业务系统运行环境、部署平台、托管平台或方案架构约束，不能要求业务方案“运行在 codex 中”。omittedRoleRationale 只能包含未被选入 teamComposition 的角色；不要要求把已选角色写入 omittedRoleRationale。tokenEstimate 可以说明初始化协作包里的规划、评审、交接文档 token 预算，只有在声称业务代码或成品已生成时才阻断。发现遗漏、矛盾、模板腔调、目标软件入口文件错误或角色取舍没有依据时必须阻断。blockingIssues 只能放真正阻断成功的问题；不要把“无问题”“不构成阻断”“仅建议”“已生成且正确”这类观察放进 blockingIssues。不要重复同一句观察。不要直接改产物，只提出 blockingIssues、questionsForMengXingxing、requiredRepairs。只返回 JSON。";
    let selected_solution = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        });
    let payload = json!({
        "round": round,
        "reviewPhase": context.phase,
        "phaseExpectations": match context.phase.as_str() {
            "pre_bootstrap_solution_review" => "This phase happens before bootstrap. generatedFiles and generatedFileEvidence are intentionally empty. Review selectedSolution semantics, role rationale, target-client handoff intent, and selected capabilities. Do not block because entry files are not generated yet. Planned target-client entry paths such as AGENTS.md/.codex are allowed in role rationale when they describe the future handoff contract, not existing evidence.",
            "final_generated_package_review" => "This phase happens after bootstrap. generatedFiles and generatedFileEvidence must prove the target-client collaboration package exists and matches the truth-source rules.",
            _ => "Review according to the provided phase and truth-source rules."
        },
        "reviewScope": "Block on selectedSolution, generated package evidence, target-client entry files, capability state, and truth-source rules. Unselected solutions are supporting context and only block if the three-solution contract is incomplete or clearly misleading.",
        "reviewerAgent": "星梦梦",
        "mainAgent": "梦星星",
        "userRequirements": {
            "understandingSummary": session.understanding_summary,
            "readiness": session.readiness,
        },
        "selectedSolutionId": session.selected_solution_id,
        "selectedSolution": selected_solution,
        "solutions": session.solutions,
        "targetClient": session_target_client(session).as_str(),
        "targetClientMeaning": "Target client controls the generated collaboration package entry only. codex means AGENTS.md/.codex handoff. It is not the business application runtime, deployment platform, hosting target, or architecture constraint.",
        "selectedCapabilities": session.selected_capabilities,
        "selectedCapabilitiesMeaning": "These are launcher and collaboration-package capabilities such as superpowers, agent-browser, chrome-devtools, GitNexus, and Speckit. They are not the business application's dependency list. Do not require app-specific auth, payment, storage, or deployment capabilities to appear here.",
        "capabilityState": session.selected_capabilities,
        "generatedFiles": context.generated_files,
        "generatedFileEvidence": context.generated_file_evidence,
        "postcheckPassed": context.postcheck_passed,
        "truthSourceRules": context.truth_source_rules,
        "schema": {
            "passed": false,
            "blockingIssues": ["string"],
            "questionsForMengXingxing": ["string"],
            "requiredRepairs": ["string"],
            "reviewSummary": "string",
            "confidence": "low|medium|high"
        }
    });
    request_semantic_json(session, prompt, &payload)
        .and_then(|value| parse_semantic_review_value(&value))
}

fn request_meng_xingxing_repair(
    session: &AgentSession,
    review: &SemanticReviewResult,
    round: usize,
) -> Result<RepairDecision, String> {
    let prompt = "你是梦星星，星星的vibecoding启动器的主初始化 Agent。星梦梦已经提出阻断项。你必须逐条回答：接受修正、补充理由，或明确拒绝并说明依据。只修正 selectedSolution；未选方案除非破坏三方案完整性，否则不要为了复核而重写。omittedRoleRationale 只能包含未被选入 teamComposition 的角色，绝对不要把“保留/不省略”的角色写进去。targetClient 只表示后续接管软件和入口文件，例如 codex 生成 AGENTS.md/.codex；不要把 codex 写成业务系统运行环境、部署平台、托管平台或架构约束。selectedCapabilities 只写启动器/协作包工作流能力；支付、数据库、AI 平台、知识库平台、部署和第三方 SDK 属于业务运行依赖，应写进方案/实施边界，不要补进 selectedCapabilities。tokenEstimate 请写成初始化协作包规划、评审、交接所需的 LLM token 预算，不要暗示业务代码已经生成。如果需要修正已选方案，请返回完整或局部 updatedSolution，且 id 必须等于 selectedSolutionId。不要伪装通过，只返回 JSON。";
    let selected_solution = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        });
    let payload = json!({
        "round": round,
        "mainAgent": "梦星星",
        "reviewerAgent": "星梦梦",
        "selectedSolutionId": session.selected_solution_id,
        "selectedSolution": selected_solution,
        "blockingIssues": review.blocking_issues,
        "questionsForMengXingxing": review.questions_for_meng_xingxing,
        "requiredRepairs": review.required_repairs,
        "schema": {
            "round": round,
            "status": "repaired|justified|rejected",
            "issues": ["string"],
            "responseSummary": "string",
            "updatedSolution": "AgentSolution or null"
        }
    });
    request_semantic_json(session, prompt, &payload)
        .and_then(|value| parse_repair_decision_value(&value, round, selected_solution))
}

fn request_semantic_json(
    session: &AgentSession,
    prompt: &str,
    payload: &Value,
) -> Result<Value, String> {
    if session_uses_codex_official_login(session) {
        return Err("codex_official_login_unsupported".to_string());
    }

    let client = Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| "semantic_agent_client_build_failed".to_string())?;
    let payload_text = serde_json::to_string_pretty(payload)
        .map_err(|_| "semantic_agent_payload_serialize_failed".to_string())?;

    let chat_endpoint = format!(
        "{}/chat/completions",
        session.base_url.trim_end_matches('/')
    );
    let chat_request = with_deepseek_chat_safety_parameters(
        session,
        json!({
            "model": session.model,
            "messages": [
                { "role": "system", "content": prompt },
                { "role": "user", "content": payload_text }
            ],
            "temperature": 0.2,
            "response_format": { "type": "json_object" }
        }),
    );
    let chat_fallback_request = with_deepseek_chat_safety_parameters(
        session,
        json!({
            "model": session.model,
            "messages": [
                { "role": "system", "content": prompt },
                { "role": "user", "content": payload_text }
            ]
        }),
    );

    let wire_api = effective_session_wire_api(session);
    let value = if wire_api == "responses" {
        let responses_endpoint = format!("{}/responses", session.base_url.trim_end_matches('/'));
        let responses_request = json!({
            "model": session.model,
            "instructions": prompt,
            "input": [{
                "type": "message",
                "role": "user",
                "id": next_message_id(),
                "content": [{ "type": "input_text", "text": payload_text }]
            }],
            "text": { "format": { "type": "json_object" } }
        });
        let responses_fallback_request = json!({
            "model": session.model,
            "input": format!("{prompt}\n\n{payload_text}")
        });

        match post_session_agent_json(
            &client,
            session,
            "semantic",
            None,
            "semantic.responses.primary",
            &responses_endpoint,
            &responses_request,
        ) {
            Ok(value) => value,
            Err(error) if error.status_code == 400 => {
                match post_session_agent_json(
                    &client,
                    session,
                    "semantic",
                    None,
                    "semantic.responses.compat",
                    &responses_endpoint,
                    &responses_fallback_request,
                ) {
                    Ok(value) => value,
                    Err(fallback_error)
                        if should_fallback_from_responses_to_chat(&fallback_error) =>
                    {
                        post_semantic_chat_json(
                            &client,
                            session,
                            &chat_endpoint,
                            &chat_request,
                            &chat_fallback_request,
                        )?
                    }
                    Err(fallback_error) => return Err(map_agent_http_error(&fallback_error)),
                }
            }
            Err(error) if should_fallback_from_responses_to_chat(&error) => {
                post_semantic_chat_json(
                    &client,
                    session,
                    &chat_endpoint,
                    &chat_request,
                    &chat_fallback_request,
                )?
            }
            Err(error) => return Err(map_agent_http_error(&error)),
        }
    } else {
        post_semantic_chat_json(
            &client,
            session,
            &chat_endpoint,
            &chat_request,
            &chat_fallback_request,
        )?
    };
    if let Some(parsed) =
        extract_json_value_from_model_response_for_provider(&value, &session.provider)
    {
        return Ok(parsed);
    }
    if let Some(recovered) = recover_semantic_review_value_from_invalid_response(&value) {
        return Ok(recovered);
    }

    let summary = summarize_semantic_invalid_response(session, &value);
    let _ = write_semantic_invalid_response_diagnostic(session, &value, &summary);
    Err(format!("semantic_agent_response_invalid: {summary}"))
}

fn summarize_semantic_invalid_response(session: &AgentSession, value: &Value) -> String {
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    let top_level = value
        .as_object()
        .map(|map| map.keys().cloned().collect::<Vec<_>>().join(","))
        .unwrap_or_else(|| value_type_name(value).to_string());
    let candidate_preview = candidates
        .iter()
        .find(|text| !text.trim().is_empty())
        .map(|text| sanitize_diagnostic_text(text))
        .unwrap_or_else(|| "no_text_candidates".to_string());
    format!(
        "provider={} model={} wireApi={} topLevelKeys={} textCandidates={} firstText={}",
        session.provider,
        session.model,
        effective_session_wire_api(session),
        top_level,
        candidates.len(),
        candidate_preview
    )
}

fn write_semantic_invalid_response_diagnostic(
    session: &AgentSession,
    value: &Value,
    summary: &str,
) -> Result<(), String> {
    let session_root = PathBuf::from(&session.workspace_path)
        .join(".commonhe")
        .join("session");
    fs::create_dir_all(&session_root).map_err(|_| "semantic_diagnostic_dir_failed".to_string())?;
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    let text_previews = candidates
        .iter()
        .take(8)
        .map(|text| sanitize_diagnostic_text(text))
        .collect::<Vec<_>>();
    let response_preview = sanitize_diagnostic_text(
        &serde_json::to_string(value).unwrap_or_else(|_| value_type_name(value).to_string()),
    );
    let diagnostic = json!({
        "error": "semantic_agent_response_invalid",
        "summary": summary,
        "provider": session.provider,
        "model": session.model,
        "wireApi": effective_session_wire_api(session),
        "topLevelType": value_type_name(value),
        "textCandidateCount": candidates.len(),
        "textCandidatePreviews": text_previews,
        "responsePreview": response_preview
    });
    fs::write(
        session_root.join("semantic-agent-invalid-response.json"),
        serde_json::to_string_pretty(&diagnostic)
            .map_err(|_| "semantic_diagnostic_serialize_failed".to_string())?,
    )
    .map_err(|_| "semantic_diagnostic_write_failed".to_string())
}

fn sanitize_diagnostic_text(text: &str) -> String {
    text.replace('\r', "\\r")
        .replace('\n', "\\n")
        .chars()
        .take(1200)
        .collect()
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn session_uses_codex_official_login(session: &AgentSession) -> bool {
    session.provider == "codex"
        && session.api_key.trim().is_empty()
        && provider::codex_official_login_available()
}

fn post_semantic_chat_json(
    client: &Client,
    session: &AgentSession,
    endpoint: &str,
    primary_request: &Value,
    fallback_request: &Value,
) -> Result<Value, String> {
    match post_session_agent_json(
        client,
        session,
        "semantic",
        None,
        "semantic.chat.primary",
        endpoint,
        primary_request,
    ) {
        Ok(value) => Ok(value),
        Err(error) if error.status_code == 400 => post_session_agent_json(
            client,
            session,
            "semantic",
            None,
            "semantic.chat.compat",
            endpoint,
            fallback_request,
        )
        .map_err(|fallback_error| map_agent_http_error(&fallback_error)),
        Err(error) => Err(map_agent_http_error(&error)),
    }
}

fn extract_json_value_from_model_response(value: &Value) -> Option<Value> {
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    for raw_content in candidates {
        if let Ok(parsed) = parse_json_candidate_value(&raw_content) {
            return Some(parsed);
        }
    }
    None
}

fn extract_json_value_from_model_response_for_provider(
    value: &Value,
    provider: &str,
) -> Option<Value> {
    if !provider_allows_minimax_reasoning_cleanup(provider) {
        return extract_json_value_from_model_response(value);
    }

    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    for raw_content in candidates {
        if let Ok(parsed) = parse_complete_json_candidate_value(&raw_content) {
            return Some(parsed);
        }
        if let Some(cleaned) = strip_reasoning_think_blocks(&raw_content) {
            if let Ok(parsed) =
                parse_json_candidate_value_embedded(&cleaned).and_then(unwind_json_string_values)
            {
                return Some(parsed);
            }
            continue;
        }

        if let Ok(parsed) = parse_json_candidate_value(&raw_content) {
            return Some(parsed);
        }
    }
    None
}

fn recover_semantic_review_value_from_invalid_response(value: &Value) -> Option<Value> {
    let mut candidates = Vec::new();
    collect_text_candidates(value, &mut candidates);
    for raw_content in candidates {
        let text = raw_content.trim();
        if text.is_empty() || find_review_issues_field_start(text).is_none() {
            continue;
        }
        if issue_explicitly_says_non_blocking(text)
            && !text.contains("必须修复")
            && !text.contains("需修复")
            && !text.contains("缺少")
            && !text.contains("矛盾")
        {
            return Some(json!({
                "passed": true,
                "blockingIssues": [],
                "questionsForMengXingxing": [],
                "requiredRepairs": [],
                "reviewSummary": "星梦梦返回的 JSON 被截断，但可见内容均自述为非阻断观察；程序按确定性规则恢复为通过。",
                "confidence": "medium"
            }));
        }

        let blocking_issues = extract_complete_blocking_issues_from_truncated_review(text);
        if !blocking_issues.is_empty() {
            return Some(json!({
                "passed": false,
                "blockingIssues": blocking_issues,
                "questionsForMengXingxing": [],
                "requiredRepairs": ["星梦梦响应被截断；程序已保留完整阻断项并要求梦星星先修复这些可见问题。"],
                "reviewSummary": "星梦梦返回的 JSON 被截断，程序已从 blockingIssues 中保留完整可解析的阻断项并按未通过处理。",
                "confidence": "medium"
            }));
        }

        if text.contains("必须修复")
            || text.contains("需修复")
            || text.contains("缺少")
            || text.contains("矛盾")
        {
            continue;
        }
    }
    None
}

fn extract_complete_blocking_issues_from_truncated_review(text: &str) -> Vec<String> {
    let Some(blocking_start) = find_review_issues_field_start(text) else {
        return Vec::new();
    };
    let Some(array_offset) = text[blocking_start..].find('[') else {
        return Vec::new();
    };
    let chars = text[blocking_start + array_offset + 1..].chars();
    let mut issues = Vec::new();
    let mut current = String::new();
    let mut in_string = false;
    let mut escaped = false;

    for ch in chars {
        if escaped {
            current.push(match ch {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                other => other,
            });
            escaped = false;
            continue;
        }

        if in_string {
            match ch {
                '\\' => escaped = true,
                '"' => {
                    let issue = current.trim().to_string();
                    if !issue.is_empty() {
                        issues.push(issue);
                    }
                    current.clear();
                    in_string = false;
                }
                other => current.push(other),
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            current.clear();
        } else if ch == ']' {
            break;
        }
    }

    issues
}

fn find_review_issues_field_start(text: &str) -> Option<usize> {
    ["\"blockingIssues\"", "\"blocking_issues\"", "\"issues\""]
        .into_iter()
        .filter_map(|field| text.find(field))
        .min()
}

fn parse_json_candidate_value(raw_content: &str) -> Result<Value, String> {
    parse_json_candidate_value_embedded(raw_content).and_then(unwind_json_string_values)
}

fn parse_complete_json_candidate_value(raw_content: &str) -> Result<Value, String> {
    serde_json::from_str::<Value>(json_candidate_without_fence(raw_content))
        .map_err(|_| "json_candidate_invalid".to_string())
        .and_then(unwind_json_string_values)
}

fn unwind_json_string_values(mut value: Value) -> Result<Value, String> {
    while let Value::String(inner) = value {
        value = parse_json_value_or_embedded_object(&inner)?;
    }
    Ok(value)
}

fn json_candidate_without_fence(raw_content: &str) -> &str {
    let trimmed = raw_content.trim();
    trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|value| value.trim_end_matches("```").trim())
        .unwrap_or(trimmed)
}

fn parse_json_candidate_value_embedded(raw_content: &str) -> Result<Value, String> {
    parse_json_value_or_embedded_object(json_candidate_without_fence(raw_content))
}

fn parse_json_value_or_embedded_object(raw_content: &str) -> Result<Value, String> {
    let trimmed = raw_content.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Ok(value);
    }
    extract_json_object(trimmed)
        .and_then(|candidate| serde_json::from_str::<Value>(&candidate).ok())
        .ok_or_else(|| "json_candidate_invalid".to_string())
}

fn looks_like_json_candidate(raw_content: &str) -> bool {
    let trimmed = raw_content.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with("```")
}

fn parse_semantic_review_value(value: &Value) -> Result<SemanticReviewResult, String> {
    if let Ok(review) = serde_json::from_value::<SemanticReviewResult>(value.clone()) {
        return Ok(review);
    }

    if !value.is_object() {
        return Err("semantic_review_response_schema_invalid".to_string());
    }

    let blocking_issues =
        string_array_field(value, &["blockingIssues", "blocking_issues", "issues"])
            .unwrap_or_default();
    if blocking_issues.is_empty() && value.get("passed").and_then(Value::as_bool).is_none() {
        return Err("semantic_review_response_schema_invalid".to_string());
    }

    let passed = value
        .get("passed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let questions_for_meng_xingxing = string_array_field(
        value,
        &[
            "questionsForMengXingxing",
            "questions_for_meng_xingxing",
            "questions",
        ],
    )
    .unwrap_or_default();
    let required_repairs =
        string_array_field(value, &["requiredRepairs", "required_repairs", "repairs"])
            .unwrap_or_default();
    let review_summary = string_field(value, &["reviewSummary", "review_summary", "summary"])
        .unwrap_or_else(|| {
            if blocking_issues.is_empty() {
                "星梦梦返回结构不完整但未提供阻断项，按 schema 缺失处理。".to_string()
            } else {
                "星梦梦返回结构不完整，程序已保留 blockingIssues 并按未通过处理。".to_string()
            }
        });
    let confidence = string_field(value, &["confidence"]).unwrap_or_else(|| "medium".to_string());

    Ok(SemanticReviewResult {
        passed,
        blocking_issues,
        questions_for_meng_xingxing,
        required_repairs,
        review_summary,
        confidence,
    })
}

fn parse_repair_decision_value(
    value: &Value,
    round: usize,
    selected_solution: Option<&AgentSolution>,
) -> Result<RepairDecision, String> {
    if !value.is_object() {
        return Err("semantic_repair_response_schema_invalid".to_string());
    }

    let repair_round = value
        .get("round")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .filter(|value| *value > 0)
        .unwrap_or(round);
    let status = string_field(value, &["status"]).unwrap_or_else(|| "repaired".to_string());
    let issues = string_array_field(value, &["issues", "blockingIssues", "blocking_issues"])
        .unwrap_or_default();
    let response_summary = string_field(
        value,
        &[
            "responseSummary",
            "response_summary",
            "summary",
            "reviewSummary",
            "review_summary",
        ],
    )
    .unwrap_or_default();
    let updated_solution = parse_updated_solution_patch(value, selected_solution)?;

    Ok(RepairDecision {
        round: repair_round,
        status,
        issues,
        response_summary,
        updated_solution,
    })
}

fn parse_updated_solution_patch(
    value: &Value,
    selected_solution: Option<&AgentSolution>,
) -> Result<Option<AgentSolution>, String> {
    for key in [
        "updatedSolution",
        "updated_solution",
        "updated",
        "solutionPatch",
        "solution_patch",
    ] {
        if let Some(candidate) = value.get(key) {
            if candidate.is_null() {
                return Ok(None);
            }
            if let Ok(solution) = serde_json::from_value::<AgentSolution>(candidate.clone()) {
                return Ok(Some(solution));
            }
            if let Some(base_solution) = selected_solution {
                if candidate.is_object() {
                    return Ok(Some(merge_agent_solution_patch(base_solution, candidate)));
                }
            }
            return Err("semantic_repair_updated_solution_invalid".to_string());
        }
    }

    if value.get("id").is_some()
        && (value.get("roleRationale").is_some()
            || value.get("role_rationale").is_some()
            || value.get("omittedRoleRationale").is_some()
            || value.get("omitted_role_rationale").is_some())
    {
        if let Ok(solution) = serde_json::from_value::<AgentSolution>(value.clone()) {
            return Ok(Some(solution));
        }
        if let Some(base_solution) = selected_solution {
            return Ok(Some(merge_agent_solution_patch(base_solution, value)));
        }
    }

    Ok(None)
}

fn merge_agent_solution_patch(base: &AgentSolution, patch: &Value) -> AgentSolution {
    let mut updated = base.clone();

    if let Some(value) = string_field(patch, &["id"]) {
        updated.id = value;
    }
    if let Some(value) = string_field(patch, &["title", "name"]) {
        updated.title = value;
    }
    if let Some(value) = string_field(
        patch,
        &[
            "architectureSummary",
            "architecture_summary",
            "architecture",
        ],
    ) {
        updated.architecture_summary = value;
    }
    if let Some(value) = string_array_field(patch, &["teamComposition", "team_composition", "team"])
    {
        updated.team_composition = value;
    }
    if let Some(value) = string_field(patch, &["tokenEstimate", "token_estimate"]) {
        updated.token_estimate = value;
    }
    if let Some(value) = string_field(
        patch,
        &[
            "recommendationText",
            "recommendation_text",
            "recommendation",
        ],
    ) {
        updated.recommendation_text = value;
    }
    if let Some(value) = string_map_field(patch, &["roleRationale", "role_rationale"]) {
        updated.role_rationale = value;
    }
    if let Some(value) =
        string_map_field(patch, &["omittedRoleRationale", "omitted_role_rationale"])
    {
        updated.omitted_role_rationale = value;
    }

    updated
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(Value::as_str)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn string_array_field(value: &Value, keys: &[&str]) -> Option<Vec<String>> {
    keys.iter().find_map(|key| {
        let candidate = value.get(*key)?;
        if let Some(items) = candidate.as_array() {
            let values = items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if values.is_empty() {
                None
            } else {
                Some(values)
            }
        } else {
            candidate
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| vec![value.to_string()])
        }
    })
}

fn string_map_field(value: &Value, keys: &[&str]) -> Option<HashMap<String, String>> {
    keys.iter().find_map(|key| {
        let object = value.get(*key)?.as_object()?;
        let values = object
            .iter()
            .filter_map(|(key, value)| {
                value
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(|value| (key.clone(), value.to_string()))
            })
            .collect::<HashMap<_, _>>();
        if values.is_empty() {
            None
        } else {
            Some(values)
        }
    })
}

fn merge_semantic_safety_floor(
    session: &AgentSession,
    mut review: SemanticReviewResult,
) -> SemanticReviewResult {
    let floor = review_selected_solution_semantics(session);
    if floor.passed {
        return review;
    }

    for issue in floor.blocking_issues {
        if !review.blocking_issues.contains(&issue) {
            review.blocking_issues.push(issue);
        }
    }
    for question in floor.questions_for_meng_xingxing {
        if !review.questions_for_meng_xingxing.contains(&question) {
            review.questions_for_meng_xingxing.push(question);
        }
    }
    for repair in floor.required_repairs {
        if !review.required_repairs.contains(&repair) {
            review.required_repairs.push(repair);
        }
    }
    if !review.blocking_issues.is_empty() {
        review.passed = false;
        if review.review_summary.trim().is_empty() || review.review_summary.contains("通过") {
            review.review_summary = "星梦梦发现阻断项，必须回传梦星星修正或说明。".to_string();
        }
    }
    review
}

fn sanitize_semantic_review_against_session(
    session: &AgentSession,
    mut review: SemanticReviewResult,
    context: &SemanticReviewContext,
) -> SemanticReviewResult {
    let original_issue_count = review.blocking_issues.len();
    review
        .blocking_issues
        .retain(|issue| !is_unsupported_semantic_blocker(session, context, issue));

    if review.blocking_issues.is_empty() && original_issue_count > 0 {
        review.passed = true;
        review.questions_for_meng_xingxing.clear();
        review.required_repairs.clear();
        review.review_summary =
            "星梦梦原始阻断项与结构化会话数据不一致，程序已按确定性规则剔除误报。".to_string();
        review.confidence = "medium".to_string();
    } else if review.passed && !review.blocking_issues.is_empty() {
        review.passed = false;
        review.review_summary =
            "星梦梦返回 passed=true 但 blockingIssues 非空，程序按阻断处理并要求回传梦星星修正或说明。".to_string();
    }

    review
}

fn is_unsupported_semantic_blocker(
    session: &AgentSession,
    context: &SemanticReviewContext,
    issue: &str,
) -> bool {
    if issue_claims_omitted_selected_role(issue)
        && !selected_solution_has_omitted_selected_role(session)
    {
        return true;
    }

    if issue_claims_business_capability_as_launcher_capability(issue) {
        return true;
    }

    if issue_requests_launcher_product_name_in_selected_solution(issue) {
        return true;
    }

    if issue_is_unmapped_natural_language_role_clarification(issue) {
        return true;
    }

    if issue_is_token_estimate_confirmation_not_blocker(issue) {
        return true;
    }

    if context.phase == "pre_bootstrap_solution_review"
        && issue_is_prebootstrap_target_entry_planning_clarification(issue)
    {
        return true;
    }

    if context.phase == "pre_bootstrap_solution_review"
        && issue_claims_prebootstrap_target_entry_plan_breaks_semantics(issue)
        && selected_solution_docs_rationale_marks_target_entry_as_plan(session)
    {
        return true;
    }

    if issue_claims_target_client_deployment_conflict(issue) {
        return true;
    }

    if issue_claims_missing_target_entry_evidence(issue)
        && context_has_target_entry_evidence(context)
    {
        return true;
    }

    if context.phase == "final_generated_package_review"
        && context.postcheck_passed == Some(true)
        && issue_claims_generated_file_preview_truncated(issue)
        && context_referenced_generated_file_exists(context, issue)
    {
        return true;
    }

    if issue_claims_missing_role_rationale(issue)
        && selected_solution_has_complete_role_rationale(session)
    {
        return true;
    }

    if issue_claims_speckit_payload_is_not_collaboration_package(issue) {
        return true;
    }

    if issue_claims_business_architecture_details_are_invalid(issue) {
        return true;
    }

    if context.phase == "final_generated_package_review"
        && context.postcheck_passed == Some(true)
        && issue_confuses_launcher_product_name_with_project_package_name(issue)
    {
        return true;
    }

    if context.phase == "final_generated_package_review"
        && context.postcheck_passed == Some(true)
        && issue_claims_missing_session_audit_artifacts(issue)
    {
        return true;
    }

    if issue_claims_ai_engineer_backend_rationale_missing(issue)
        && selected_solution_backend_rationale_mentions_ai(session)
    {
        return true;
    }

    if issue_claims_missing_codex_entry_reference(issue)
        && context_agents_entry_references_codex(context)
    {
        return true;
    }

    if context.phase == "final_generated_package_review"
        && context.postcheck_passed == Some(true)
        && issue_claims_template_architecture_without_deterministic_failure(issue)
    {
        return true;
    }

    if context.phase == "final_generated_package_review"
        && context.postcheck_passed == Some(true)
        && issue_is_postcheck_passed_template_advice(issue)
    {
        return true;
    }

    if issue_explicitly_says_non_blocking(issue) {
        return true;
    }

    if issue_explicitly_says_no_problem(issue) {
        return true;
    }

    false
}

fn issue_claims_omitted_selected_role(issue: &str) -> bool {
    issue.contains("omittedRoleRationale")
        && (issue.contains("已选")
            || issue.contains("teamComposition")
            || issue.contains("不应出现在"))
}

fn selected_solution_has_omitted_selected_role(session: &AgentSession) -> bool {
    let Some(selected_solution) = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        })
    else {
        return false;
    };

    let selected_roles = selected_solution
        .team_composition
        .iter()
        .chain(selected_solution.role_rationale.keys())
        .map(|role| normalize_role_key(role))
        .collect::<Vec<_>>();

    selected_solution
        .omitted_role_rationale
        .keys()
        .map(|role| normalize_role_key(role))
        .any(|omitted_role| selected_roles.iter().any(|role| role == &omitted_role))
}

fn issue_claims_business_capability_as_launcher_capability(issue: &str) -> bool {
    issue.contains("selectedCapabilities")
        && (issue.contains("业务依赖")
            || issue.contains("外部业务依赖")
            || issue.contains("能力清单与架构方案矛盾")
            || issue.contains("能力清单与方案描述不匹配")
            || issue.contains("AI")
            || issue.contains("知识库")
            || issue.contains("支付")
            || issue.contains("微信支付")
            || issue.contains("支付宝")
            || issue.contains("Supabase")
            || issue.contains("数据库")
            || issue.contains("baidu-knowledge-base")
            || issue.contains("tongyi-api")
            || issue.contains("wechat-pay")
            || issue.contains("alipay")
            || issue.contains("SDK")
            || issue.contains("Vercel")
            || issue.contains("Prisma")
            || issue.contains("Postgres")
            || issue.contains("认证")
            || issue.contains("auth0")
            || issue.contains("session")
            || issue.contains("Session"))
}

fn issue_requests_launcher_product_name_in_selected_solution(issue: &str) -> bool {
    issue.contains("星星的vibecoding启动器")
        && (issue.contains("产品主名称") || issue.contains("产品名") || issue.contains("主名称"))
        && (issue.contains("selectedSolution") || issue.contains("architectureSummary"))
        && (issue.contains("未提及")
            || issue.contains("是否已正确设置")
            || issue.contains("补充产品主名称")
            || issue.contains("启动器对外名称"))
}

fn issue_is_unmapped_natural_language_role_clarification(issue: &str) -> bool {
    issue.contains("omittedRoleRationale")
        && issue.contains("未映射")
        && issue.contains("并入标准协作角色")
        && (issue.contains("过于模糊")
            || issue.contains("未说明具体并入")
            || issue.contains("取舍必须有依据"))
}

fn issue_is_token_estimate_confirmation_not_blocker(issue: &str) -> bool {
    issue.contains("tokenEstimate")
        && (issue.contains("当前表述合理")
            || issue.contains("当前表述虽声明不包含业务代码")
            || issue.contains("不包含业务代码")
            || issue.contains("可能被误解")
            || issue.contains("需确认")
            || issue.contains("仅指协作包")
            || issue.contains("计划中的 token 预算")
            || issue.contains("规划、评审、交接文档")
            || (issue.contains("重复出现")
                && issue.contains("未区分")
                && issue.contains("可能引起混淆")))
}

fn issue_is_prebootstrap_target_entry_planning_clarification(issue: &str) -> bool {
    (issue.contains("AGENTS.md") || issue.contains(".codex") || issue.contains("targetClient"))
        && issue.contains("pre_bootstrap")
        && issue.contains("尚未生成")
        && (issue.contains("预期行为")
            || issue.contains("规划中的文件结构")
            || issue.contains("建议明确说明")
            || issue.contains("不应承诺具体文件路径")
            || issue.contains("可能误导后续阶段")
            || issue.contains("建议改为描述职责")
            || issue.contains("移除具体文件路径"))
}

fn issue_claims_prebootstrap_target_entry_plan_breaks_semantics(issue: &str) -> bool {
    (issue.contains("AGENTS.md") || issue.contains(".codex") || issue.contains("targetClient"))
        && issue.contains("pre_bootstrap")
        && issue.contains("尚未生成")
        && (issue.contains("违反 targetClient")
            || issue.contains("暗示文件已存在")
            || issue.contains("作为业务运行环境")
            || issue.contains("描述角色职责而非具体文件路径"))
}

fn selected_solution_docs_rationale_marks_target_entry_as_plan(session: &AgentSession) -> bool {
    let Some(selected_solution) = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        })
    else {
        return false;
    };

    selected_solution
        .role_rationale
        .iter()
        .any(|(role, rationale)| {
            role_label_matches_canonical(role, "docs")
                && (rationale.contains("计划")
                    || rationale.contains("后续阶段")
                    || rationale.contains("后续生成")
                    || rationale.contains("交接契约"))
                && (rationale.contains("AGENTS.md") || rationale.contains(".codex"))
                && !rationale.contains("已生成")
                && !rationale.contains("已经生成")
                && !rationale.contains("已存在")
        })
}

fn issue_claims_target_client_deployment_conflict(issue: &str) -> bool {
    issue.contains("targetClient")
        && issue.contains("codex")
        && (issue.contains("部署") || issue.contains("Vercel") || issue.contains("托管"))
}

fn issue_claims_missing_target_entry_evidence(issue: &str) -> bool {
    (issue.contains("AGENTS.md") || issue.contains(".codex") || issue.contains("targetClient"))
        && (issue.contains("contentPreview")
            || issue.contains("证据")
            || issue.contains("无法验证")
            || issue.contains("缺少"))
}

fn context_has_target_entry_evidence(context: &SemanticReviewContext) -> bool {
    let generated_files_text = context
        .generated_files
        .join("\n")
        .replace('\\', "/")
        .to_lowercase();
    let evidence_text = context
        .generated_file_evidence
        .iter()
        .map(|item| item.relative_path.replace('\\', "/").to_lowercase())
        .collect::<Vec<_>>()
        .join("\n");

    generated_files_text.contains("agents.md")
        && generated_files_text.contains("/.codex/")
        && evidence_text.contains("agents.md")
        && evidence_text.contains("/.codex/")
}

fn issue_claims_generated_file_preview_truncated(issue: &str) -> bool {
    issue.contains("generatedFileEvidence")
        && issue.contains("contentPreview")
        && (issue.contains("截断") || issue.contains("无法确认文件完整性"))
}

fn context_referenced_generated_file_exists(context: &SemanticReviewContext, issue: &str) -> bool {
    context.generated_file_evidence.iter().any(|item| {
        let relative_path = item.relative_path.replace('\\', "/");
        issue.contains(&relative_path) && Path::new(&item.path).is_file()
    })
}

fn issue_claims_missing_role_rationale(issue: &str) -> bool {
    issue.contains("梦星星")
        && issue.contains("角色")
        && issue.contains("理由")
        && (issue.contains("没有说明") || issue.contains("未说明") || issue.contains("补齐"))
}

fn selected_solution_has_complete_role_rationale(session: &AgentSession) -> bool {
    let Some(selected_solution) = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        })
    else {
        return false;
    };
    if selected_solution.team_composition.is_empty() || selected_solution.role_rationale.is_empty()
    {
        return false;
    }

    selected_solution.team_composition.iter().all(|role| {
        selected_solution
            .role_rationale
            .iter()
            .any(|(rationale_role, rationale)| {
                role_label_matches_canonical(rationale_role, role) && !rationale.trim().is_empty()
            })
    }) && selected_solution
        .omitted_role_rationale
        .values()
        .all(|rationale| !rationale.trim().is_empty())
}

fn issue_claims_speckit_payload_is_not_collaboration_package(issue: &str) -> bool {
    issue.contains(".specify")
        && (issue.contains("内部文件")
            || issue.contains("通用模板")
            || issue.contains("非协作包")
            || issue.contains("不属于目标客户端"))
}

fn issue_claims_business_architecture_details_are_invalid(issue: &str) -> bool {
    issue.contains("architectureSummary")
        && (issue.contains("业务技术栈")
            || issue.contains("业务实现细节")
            || issue.contains("Next.js")
            || issue.contains("Prisma"))
}

fn issue_confuses_launcher_product_name_with_project_package_name(issue: &str) -> bool {
    issue.contains("星星的vibecoding启动器")
        && (issue.contains("产品主名称") || issue.contains("产品名") || issue.contains("主名称"))
        && (issue.contains("所有文档")
            || issue.contains("项目名")
            || issue.contains("协作包")
            || issue.contains("均使用")
            || issue.contains("未出现"))
}

fn issue_claims_missing_session_audit_artifacts(issue: &str) -> bool {
    issue.contains("generatedFiles")
        && (issue.contains(".commonhe/session")
            || issue.contains("session 审计")
            || issue.contains("decision.json")
            || issue.contains("meng-xingxing-output.json")
            || issue.contains("final-acceptance.json")
            || issue.contains("status.json"))
        && (issue.contains("缺少") || issue.contains("不存在"))
}

fn issue_claims_ai_engineer_backend_rationale_missing(issue: &str) -> bool {
    issue.contains("engineering-ai-engineer")
        && issue.contains("backend")
        && (issue.contains("AI") || issue.contains("大模型") || issue.contains("导购问答"))
        && (issue.contains("未明确")
            || issue.contains("需在 backend")
            || issue.contains("显式说明"))
}

fn selected_solution_backend_rationale_mentions_ai(session: &AgentSession) -> bool {
    let Some(selected_solution) = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        })
    else {
        return false;
    };

    selected_solution
        .role_rationale
        .get("backend")
        .map(|rationale| {
            rationale.contains("AI")
                || rationale.contains("大模型")
                || rationale.contains("导购问答")
                || rationale.to_ascii_lowercase().contains("llm")
        })
        .unwrap_or(false)
}

fn issue_claims_missing_codex_entry_reference(issue: &str) -> bool {
    let issue_lower = issue.to_ascii_lowercase();
    issue.contains("AGENTS.md")
        && (issue_lower.contains("codex") || issue.contains("targetClient"))
        && (issue.contains("未包含")
            || issue.contains("缺少")
            || issue.contains("没有")
            || issue.contains("未明确")
            || issue.contains("无法验证")
            || issue.contains("需确认")
            || issue.contains("是否满足")
            || issue.contains("入口文件")
            || issue.contains("原生入口"))
}

fn context_agents_entry_references_codex(context: &SemanticReviewContext) -> bool {
    context.generated_file_evidence.iter().any(|item| {
        item.relative_path
            .replace('\\', "/")
            .eq_ignore_ascii_case("AGENTS.md")
            && {
                let preview = item.content_preview.replace('\\', "/");
                let preview_lower = preview.to_ascii_lowercase();
                preview.contains(".codex/")
                    || preview.contains(".codex")
                    || preview.contains("COORDINATOR-SUBAGENTS.md")
                    || (preview_lower.contains("codex")
                        && (preview.contains("原生入口") || preview.contains("重新接手")))
            }
    })
}

fn issue_claims_template_architecture_without_deterministic_failure(issue: &str) -> bool {
    (issue.contains("模板腔调") || issue.contains("模板"))
        && (issue.contains("技术栈") || issue.contains("架构") || issue.contains("架构细节"))
}

fn issue_is_postcheck_passed_template_advice(issue: &str) -> bool {
    let points_to_workflow_template = issue.contains("first-sprint-contract")
        || issue.contains("sprint-contract-template")
        || issue.contains("docs/workflow/");
    let points_to_agent_handbook = issue.contains("docs/agents/") || issue.contains("-handbook.md");
    (points_to_workflow_template || points_to_agent_handbook)
        && (issue.contains("模板")
            || issue.contains("重复")
            || issue.contains("过早")
            || issue.contains("复杂性")
            || issue.contains("当前初始化协作包阶段")
            || issue.contains("尚未进入实施"))
}

fn issue_explicitly_says_non_blocking(issue: &str) -> bool {
    issue.contains("不阻断")
        || issue.contains("不阻塞")
        || issue.contains("无阻断")
        || issue.contains("无阻塞")
        || issue.contains("非阻断")
        || issue.contains("非阻塞")
        || issue.contains("无需阻断")
        || issue.contains("无需阻塞")
        || issue.contains("不能因此阻断")
        || issue.contains("不能因此阻塞")
        || issue.contains("不构成阻断")
        || issue.contains("不构成阻塞")
        || issue.contains("不作为阻断")
        || issue.contains("不作为阻塞")
        || issue.contains("符合阶段要求")
        || issue.contains("符合规则要求")
        || issue.contains("符合规则")
        || issue.contains("作为计划允许")
        || issue.contains("可接受")
        || issue.contains("合理。")
        || issue.contains("合理，")
        || issue.contains("无矛盾")
        || ((issue.contains("一致") || issue.contains("已包含"))
            && !issue.contains("不一致")
            && !issue.contains("存在矛盾")
            && !issue.contains("构成矛盾"))
}

fn issue_explicitly_says_no_problem(issue: &str) -> bool {
    (issue.contains("未发现")
        && (issue.contains("问题")
            || issue.contains("矛盾")
            || issue.contains("遗漏")
            || issue.contains("无依据")
            || issue.contains("模板腔调")))
        || issue.contains("未破坏完整性")
        || (issue.contains("与用户需求一致") && issue.contains("未发现"))
        || issue.contains("无问题")
}

fn review_selected_solution_semantics(session: &AgentSession) -> SemanticReviewResult {
    let selected_solution = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        });
    let mut blocking_issues = Vec::new();
    let mut questions = Vec::new();
    let mut repairs = Vec::new();

    if selected_solution.is_none() {
        blocking_issues.push("梦星星没有可复核的已选方案。".to_string());
    }

    if let Some(solution) = selected_solution {
        if solution.role_rationale.is_empty() || solution.omitted_role_rationale.is_empty() {
            blocking_issues.push("梦星星没有说明角色选择理由和明显候选角色不选理由。".to_string());
            questions.push("请逐条说明当前团队中每个角色为什么需要，以及 qa/reviewer/database/security/devops 等明显候选角色为什么暂不需要。".to_string());
            repairs.push(
                "补齐 roleRationale 与 omittedRoleRationale 后重新交给星梦梦复核。".to_string(),
            );
        }

        let delivery_mode = infer_delivery_mode(session);
        let team_text = solution.team_composition.join(" ").to_lowercase();
        let rationale_text = format!(
            "{} {}",
            solution
                .role_rationale
                .iter()
                .map(|(role, reason)| format!("{role}:{reason}"))
                .collect::<Vec<_>>()
                .join(" "),
            solution
                .omitted_role_rationale
                .iter()
                .map(|(role, reason)| format!("{role}:{reason}"))
                .collect::<Vec<_>>()
                .join(" ")
        )
        .to_lowercase();
        if delivery_mode == "web-miniapp" {
            for role in ["qa", "reviewer"] {
                if !team_text.contains(role) && !rationale_text.contains(role) {
                    blocking_issues.push(format!(
                        "Web + 小程序方案没有让梦星星解释 `{role}` 是否需要。"
                    ));
                    questions.push(format!("结合 Web、小程序、API 和关键状态一致性，你确认 `{role}` 不需要进入当前团队吗？"));
                    repairs.push(format!("补充 `{role}` 的选择理由或不选理由。"));
                }
            }
        }
    }

    let passed = blocking_issues.is_empty();
    SemanticReviewResult {
        passed,
        blocking_issues,
        questions_for_meng_xingxing: questions,
        required_repairs: repairs,
        review_summary: if passed {
            "星梦梦已完成语义验收，未发现阻断项。".to_string()
        } else {
            "星梦梦发现阻断项，必须回传梦星星修正或说明。".to_string()
        },
        confidence: if passed { "high" } else { "medium" }.to_string(),
    }
}

fn write_semantic_review_artifacts(
    session: &AgentSession,
    session_root: &Path,
    review: &SemanticReviewResult,
    dialogue_rounds: &[AgentDialogueRound],
) -> Result<(), String> {
    fs::write(
        session_root.join("meng-xingxing-output.json"),
        serde_json::to_string_pretty(&json!({
            "mainAgent": "梦星星",
            "understandingSummary": session.understanding_summary,
            "solutions": session.solutions,
            "selectedSolutionId": session.selected_solution_id,
        }))
        .map_err(|_| "semantic_meng_xingxing_output_serialize_failed".to_string())?,
    )
    .map_err(|_| "semantic_meng_xingxing_output_write_failed".to_string())?;

    fs::write(
        session_root.join("xing-mengmeng-review.json"),
        serde_json::to_string_pretty(review)
            .map_err(|_| "semantic_review_serialize_failed".to_string())?,
    )
    .map_err(|_| "semantic_review_write_failed".to_string())?;

    let dialogue_text = dialogue_rounds
        .iter()
        .map(|round| {
            serde_json::to_string(round)
                .map_err(|_| "semantic_dialogue_serialize_failed".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    fs::write(
        session_root.join("agent-dialogue-rounds.jsonl"),
        format!("{dialogue_text}\n"),
    )
    .map_err(|_| "semantic_dialogue_write_failed".to_string())?;

    let repair_decisions = dialogue_rounds
        .iter()
        .filter_map(|round| round.repair.clone())
        .collect::<Vec<_>>();
    fs::write(
        session_root.join("repair-decisions.json"),
        serde_json::to_string_pretty(&repair_decisions)
            .map_err(|_| "semantic_repair_serialize_failed".to_string())?,
    )
    .map_err(|_| "semantic_repair_write_failed".to_string())?;

    let target_client = session_target_client(session);
    fs::write(
        session_root.join("final-acceptance.json"),
        serde_json::to_string_pretty(&json!({
            "passed": review.passed,
            "reviewerAgent": "星梦梦",
            "mainAgent": "梦星星",
            "blockingIssues": review.blocking_issues,
            "acceptedAt": started_at_string(),
            "targetClient": target_client.as_str(),
            "selectedSolutionId": session.selected_solution_id,
            "reviewRounds": dialogue_rounds.len(),
        }))
        .map_err(|_| "semantic_acceptance_serialize_failed".to_string())?,
    )
    .map_err(|_| "semantic_acceptance_write_failed".to_string())?;

    let status_path = session_root.join("status.json");
    if status_path.is_file() {
        let mut status_value = fs::read_to_string(&status_path)
            .ok()
            .and_then(|content| serde_json::from_str::<Value>(&content).ok())
            .unwrap_or_else(|| json!({}));
        if let Value::Object(status) = &mut status_value {
            status.insert(
                "semantic_review_passed".to_string(),
                Value::Bool(review.passed),
            );
            status.insert(
                "semantic_review_failed".to_string(),
                Value::Bool(!review.passed),
            );
            status.insert(
                "semantic_review_issues".to_string(),
                Value::Array(
                    review
                        .blocking_issues
                        .iter()
                        .map(|issue| Value::String(issue.clone()))
                        .collect(),
                ),
            );
            status.insert(
                "semantic_review_rounds".to_string(),
                json!(dialogue_rounds.len()),
            );
        }
        fs::write(
            status_path,
            serde_json::to_string_pretty(&status_value)
                .map_err(|_| "semantic_status_serialize_failed".to_string())?,
        )
        .map_err(|_| "semantic_status_write_failed".to_string())?;
    }

    Ok(())
}

fn prepare_bootstrap_session(session: &AgentSession) -> Result<PathBuf, String> {
    let session_root = PathBuf::from(&session.workspace_path)
        .join(".commonhe")
        .join("session");
    fs::create_dir_all(&session_root)
        .map_err(|_| "desktop_session_root_create_failed".to_string())?;
    let _project_name = session_project_name(session)?;
    let target_client = session_target_client(session);
    let selected_capabilities =
        resolve_selected_capabilities(session.selected_capabilities.clone());

    let answers = build_answers_json(session);
    fs::write(
        session_root.join("answers.json"),
        serde_json::to_string_pretty(&answers)
            .map_err(|_| "desktop_answers_serialize_failed".to_string())?,
    )
    .map_err(|_| "desktop_answers_write_failed".to_string())?;

    let proposal_options = build_proposal_options(session);
    fs::write(
        session_root.join("proposal-options.json"),
        serde_json::to_string_pretty(&proposal_options)
            .map_err(|_| "desktop_proposal_options_serialize_failed".to_string())?,
    )
    .map_err(|_| "desktop_proposal_options_write_failed".to_string())?;

    let decision = build_decision_seed(session);
    fs::write(
        session_root.join("decision.json"),
        serde_json::to_string_pretty(&decision)
            .map_err(|_| "desktop_decision_serialize_failed".to_string())?,
    )
    .map_err(|_| "desktop_decision_write_failed".to_string())?;

    let status = json!({
        "stage": "proposal",
        "current_question_index": 0,
        "session_root": session_root.to_string_lossy().to_string(),
        "question_source": "desktop-agent",
        "started_at": started_at_string(),
        "precheck_passed": false,
        "precheck_failed": false,
        "capability_gate_passed": Value::Null,
        "init_closed": false,
        "closure_gate_active": false,
        "target_client": target_client.as_str(),
        "selected_capabilities": selected_capabilities,
        "semantic_review_passed": Value::Null,
        "semantic_review_failed": Value::Null,
        "semantic_review_issues": [],
        "semantic_review_rounds": 0,
        "last_postcheck_passed": Value::Null
    });
    fs::write(
        session_root.join("status.json"),
        serde_json::to_string_pretty(&status)
            .map_err(|_| "desktop_status_serialize_failed".to_string())?,
    )
    .map_err(|_| "desktop_status_write_failed".to_string())?;

    let proposal_markdown = session
        .solutions
        .iter()
        .map(|solution| format!("## 方案 {}\n{}\n", solution.id, solution.title))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(session_root.join("proposal.md"), proposal_markdown)
        .map_err(|_| "desktop_proposal_write_failed".to_string())?;

    Ok(session_root)
}

fn started_at_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn build_answers_json(session: &AgentSession) -> Value {
    let project_name =
        session_project_name(session).unwrap_or_else(|_| "unnamed-init-package".to_string());
    let latest_user_summary = session
        .messages
        .iter()
        .filter(|message| message.role == "user")
        .map(|message| message.content.clone())
        .collect::<Vec<_>>()
        .join("\n");

    json!({
        "project_name": project_name,
        "project_goal": session.understanding_summary.clone().unwrap_or_else(|| latest_user_summary.clone()),
        "target_users": session.readiness.target_users.clone().unwrap_or_else(|| "待通过后续实施细化".to_string()),
        "core_features": if session.readiness.key_features.is_empty() {
            latest_user_summary.clone()
        } else {
            session.readiness.key_features.join("、")
        },
        "constraints": if session.readiness.constraints.is_empty() {
            "需先通过初始化、落盘与 postcheck".to_string()
        } else {
            session.readiness.constraints.join("、")
        }
    })
}

fn build_decision_seed(session: &AgentSession) -> Value {
    let project_name =
        session_project_name(session).unwrap_or_else(|_| "unnamed-init-package".to_string());
    let target_client = session_target_client(session);
    let selected_capabilities =
        resolve_selected_capabilities(session.selected_capabilities.clone());
    let selected_solution = session
        .selected_solution_id
        .as_ref()
        .and_then(|solution_id| {
            session
                .solutions
                .iter()
                .find(|solution| &solution.id == solution_id)
        })
        .cloned()
        .map(|mut solution| {
            normalize_repaired_solution_rationale(&mut solution);
            solution
        });
    let selected_solution_id = session.selected_solution_id.clone().unwrap_or_default();
    let selected_solution_title = selected_solution
        .as_ref()
        .map(|solution| solution.title.clone())
        .unwrap_or_default();
    let selected_solution_architecture = selected_solution
        .as_ref()
        .map(|solution| solution.architecture_summary.clone())
        .unwrap_or_default();
    let selected_solution_team = selected_solution
        .as_ref()
        .map(|solution| solution.team_composition.clone())
        .unwrap_or_default();
    let selected_solution_token_estimate = selected_solution
        .as_ref()
        .map(|solution| solution.token_estimate.clone())
        .unwrap_or_default();
    let selected_solution_recommendation = selected_solution
        .as_ref()
        .map(|solution| solution.recommendation_text.clone())
        .unwrap_or_default();
    let selected_solution_role_rationale = selected_solution
        .as_ref()
        .map(|solution| solution.role_rationale.clone())
        .unwrap_or_default();
    let selected_solution_omitted_role_rationale = selected_solution
        .as_ref()
        .map(|solution| solution.omitted_role_rationale.clone())
        .unwrap_or_default();
    json!({
        "user_confirmed": false,
        "auto_confirmed": false,
        "confirmation_mode": "",
        "discovery_mode": "",
        "project_name": project_name,
        "target_client": target_client.as_str(),
        "selected_capabilities": selected_capabilities,
        "selected_solution_id": selected_solution_id,
        "selected_solution_title": selected_solution_title,
        "project_type": "",
        "delivery_mode": "",
        "solution_mode": "",
        "enabled_roles": [],
        "recommended_roles_now": [],
        "available_roles_later": [],
        "integrations": [],
        "detected_integrations": [],
        "external_references": [],
        "current_stage": "",
        "current_stage_goal": "",
        "primary_workstream": "",
        "stage_constraints": [],
        "deferred_capabilities": [],
        "implementation_checklist_seed": [],
        "implementation_acceptance_seed": [],
        "required_capabilities": [],
        "capability_probe_results": [],
        "legacy_analysis_version": "",
        "signal_categories": [],
        "role_rationale": selected_solution_role_rationale,
        "omitted_role_rationale": selected_solution_omitted_role_rationale,
        "confidence_breakdown": {},
        "dominant_workstreams": [],
        "kickoff_pack": {},
        "analysis_confidence": "",
        "autodiscovery_signals": [],
        "autodiscovery_assumptions": [],
        "solution_architecture_summary": selected_solution_architecture,
        "solution_team_composition": selected_solution_team,
        "solution_token_estimate": selected_solution_token_estimate,
        "solution_recommendation_text": selected_solution_recommendation,
        "project_goal_summary": session.understanding_summary.clone().unwrap_or_default(),
        "target_users_summary": session.readiness.target_users.clone().unwrap_or_default(),
        "core_features_summary": if session.readiness.key_features.is_empty() {
            "".to_string()
        } else {
            session.readiness.key_features.join("、")
        },
        "constraints_summary": if session.readiness.constraints.is_empty() {
            "".to_string()
        } else {
            session.readiness.constraints.join("、")
        }
    })
}

fn build_proposal_options(session: &AgentSession) -> Vec<Value> {
    session
        .solutions
        .iter()
        .enumerate()
        .map(|(index, solution)| build_proposal_option(session, solution, index))
        .collect()
}

fn build_proposal_option(session: &AgentSession, solution: &AgentSolution, index: usize) -> Value {
    let inferred_mode = infer_solution_mode(solution, index);
    let delivery_mode = infer_delivery_mode(session);
    let enabled_roles = infer_enabled_roles(solution, &inferred_mode, Some(&delivery_mode));
    let available_roles_later = infer_later_roles(solution)
        .into_iter()
        .filter(|role| !enabled_roles.iter().any(|enabled| enabled == role))
        .collect::<Vec<_>>();
    let project_type = infer_project_type(session);
    let current_stage_goal = session
        .understanding_summary
        .clone()
        .unwrap_or_else(|| "围绕已选方案建立可验证的初始化闭环。".to_string());

    json!({
        "id": solution.id,
        "name": solution.title,
        "architecture_summary": solution.architecture_summary,
        "team_composition": enabled_roles.clone(),
        "agent_authored_team_composition": solution.team_composition,
        "token_estimate": solution.token_estimate,
        "recommendation_text": solution.recommendation_text,
        "role_rationale": solution.role_rationale.clone(),
        "omitted_role_rationale": solution.omitted_role_rationale.clone(),
        "project_type": project_type,
        "delivery_mode": delivery_mode,
        "solution_mode": inferred_mode,
        "enabled_roles": enabled_roles.clone(),
        "recommended_roles_now": enabled_roles,
        "available_roles_later": available_roles_later,
        "integrations": [],
        "detected_integrations": [],
        "external_references": [],
        "current_stage": "implementation",
        "current_stage_goal": current_stage_goal,
        "primary_workstream": infer_primary_workstream(solution),
        "stage_constraints": ["必须保持桌面主流程真实、可验证、可收口"],
        "deferred_capabilities": [],
        "implementation_checklist_seed": [
            "阅读目标软件入口和 docs 真源",
            "确认首轮实施范围、责任角色与验证证据"
        ],
        "implementation_acceptance_seed": [
            "初始化协作包结构、目标软件入口和 session 审计产物已通过 postcheck",
            "postcheck 通过后才能宣布成功"
        ]
    })
}

fn infer_solution_mode(solution: &AgentSolution, index: usize) -> String {
    match solution.id.trim().to_uppercase().as_str() {
        "A" => "fast-mvp".to_string(),
        "B" => "balanced".to_string(),
        "C" => "enterprise".to_string(),
        _ => match index {
            0 => "fast-mvp".to_string(),
            1 => "balanced".to_string(),
            _ => "enterprise".to_string(),
        },
    }
}

fn normalize_selected_solution_for_package_roles(session: &mut AgentSession, solution_id: &str) {
    let delivery_mode = infer_delivery_mode(session);
    if let Some((index, solution)) = session
        .solutions
        .iter_mut()
        .enumerate()
        .find(|(_, solution)| solution.id == solution_id)
    {
        let solution_mode = infer_solution_mode(solution, index);
        *solution =
            align_solution_with_package_roles(solution, &solution_mode, Some(&delivery_mode));
    }
}

fn align_solution_with_package_roles(
    solution: &AgentSolution,
    solution_mode: &str,
    delivery_mode: Option<&str>,
) -> AgentSolution {
    let enabled_roles = infer_enabled_roles(solution, solution_mode, delivery_mode);
    let mut aligned = solution.clone();
    aligned.team_composition = enabled_roles.clone();

    let mut role_rationale = HashMap::new();
    for role in &enabled_roles {
        let rationale = solution
            .role_rationale
            .iter()
            .find(|(original_role, _)| role_label_matches_canonical(original_role, role))
            .map(|(_, rationale)| rationale.clone())
            .unwrap_or_else(|| default_package_role_rationale(role, solution));
        role_rationale.insert(role.clone(), rationale);
    }
    aligned.role_rationale = role_rationale;

    let mut omitted_role_rationale = HashMap::new();
    for (role, rationale) in &solution.omitted_role_rationale {
        if enabled_roles
            .iter()
            .any(|enabled| role_label_matches_canonical(role, enabled))
        {
            continue;
        }
        omitted_role_rationale.insert(role.clone(), rationale.clone());
    }
    for role in &solution.team_composition {
        if enabled_roles
            .iter()
            .any(|enabled| role_label_matches_canonical(role, enabled))
        {
            continue;
        }
        omitted_role_rationale
            .entry(role.clone())
            .or_insert_with(|| omitted_rationale_for_unenabled_role(solution, role));
    }
    omitted_role_rationale
        .entry("产品经理".to_string())
        .or_insert_with(|| default_omitted_package_role_rationale("产品经理"));
    aligned.omitted_role_rationale = omitted_role_rationale;

    normalize_repaired_solution_rationale(&mut aligned);
    aligned
}

fn role_label_matches_canonical(label: &str, canonical_role: &str) -> bool {
    normalize_role_key(&canonical_role_from_label(label)) == normalize_role_key(canonical_role)
}

fn canonical_role_from_label(label: &str) -> String {
    let lower = label.to_lowercase();
    if lower.contains("frontend") || label.contains("前端") || label.contains("页面") {
        return "frontend".to_string();
    }
    if lower.contains("backend") || label.contains("后端") || label.contains("服务端") {
        return "backend".to_string();
    }
    if lower.contains("devops")
        || lower.contains("sre")
        || label.contains("部署")
        || label.contains("运维")
    {
        return "devops".to_string();
    }
    if lower.contains("docs") || label.contains("文档") || label.contains("技术写作") {
        return "docs".to_string();
    }
    if lower.contains("review") || label.contains("审查") || label.contains("代码审查") {
        return "reviewer".to_string();
    }
    if lower.contains("qa")
        || lower.contains("test")
        || label.contains("测试")
        || label.contains("验收")
    {
        return "qa".to_string();
    }
    if lower.contains("architect") || label.contains("架构") {
        return "architect".to_string();
    }
    if lower.contains("database")
        || lower.contains("data")
        || label.contains("数据库")
        || label.contains("数据")
    {
        return "database".to_string();
    }
    if lower.contains("miniapp") || lower.contains("wechat") || label.contains("小程序") {
        return "miniapp".to_string();
    }
    if lower.contains("security")
        || lower.contains("compliance")
        || label.contains("安全")
        || label.contains("合规")
    {
        return "compliance".to_string();
    }
    if lower.contains("product") || label.contains("产品经理") || label.contains("pm") {
        return "product-manager".to_string();
    }
    label.to_string()
}

fn default_package_role_rationale(role: &str, solution: &AgentSolution) -> String {
    match role {
        "frontend" => "承接梦星星方案中的页面、交互、用户可见路径和基础 UI/UX 取舍；使用选定组件库保证一致性。".to_string(),
        "backend" => "承接接口、数据真源和外部服务集成边界；即使采用托管或低代码后端，也需要在协作包中明确后端责任。".to_string(),
        "docs" => "当前负责维护初始化协作包真源、方案决策、角色职责和语义验收记录；后续计划在 bootstrap 阶段生成目标软件接管入口文件 AGENTS.md 与 .codex/，实际落盘由启动器 Codex 模板执行，docs 负责记录交接说明和引用路径，不声称当前文件已经落盘。".to_string(),
        "reviewer" => "作为目标软件协作包内的后续审查角色，负责实施阶段的语义复核、角色取舍一致性和方案范围漂移检查；星梦梦是启动器本轮初始化验收 Agent，二者阶段不同，reviewer 不替代星梦梦，也不承接 QA 执行职责。".to_string(),
        "devops" => "承接部署、环境变量、CI/CD 和上线回滚边界。".to_string(),
        "architect" => "把梦星星方案中的架构取舍沉淀为可执行实施边界。".to_string(),
        "database" => "负责数据模型、状态一致性、迁移和关键查询边界。".to_string(),
        "qa" => "负责可执行测试计划、关键路径验证、技术回归、跨端一致性和缺陷证据；不替代 reviewer 的语义/范围复核。".to_string(),
        "miniapp" => "负责微信小程序端页面、端侧状态和跨端交互边界。".to_string(),
        "compliance" => "负责安全、合规、权限和敏感数据处理边界。".to_string(),
        _ => format!("承接 {} 中对应的协作职责。", solution.title),
    }
}

fn default_package_token_estimate() -> String {
    "初始化协作包 LLM 预算约 12-16 万 token：需求/方案梳理约 3 万，角色职责与边界约 2-3 万，星梦梦语义验收与修复约 3-4 万，目标软件接管入口与交接文档约 4-5 万，预留复杂度缓冲约 1 万；不包含业务代码生成或业务实现验收。".to_string()
}

fn default_complex_package_token_estimate(solution: &AgentSolution) -> String {
    let role_count = solution
        .team_composition
        .len()
        .max(solution.role_rationale.len());
    format!(
        "初始化协作包 LLM 预算约 14-18 万 token：需求/方案复核约 3 万，{role_count} 个角色职责与边界约 3 万，跨端/低代码/集成约束说明约 2 万，星梦梦语义验收与修复约 3-5 万，目标软件接管入口与交接文档约 3-4 万，预留复杂度缓冲约 1 万；不包含业务代码生成或业务实现验收。"
    )
}

fn default_omitted_package_role_rationale(role: &str) -> String {
    if canonical_role_from_label(role) == "product-manager" {
        "当前目标软件协作包不生成独立产品经理 Agent；后续迭代中的产品决策职责由 architect 兼任，reviewer 在验收时复核范围漂移。".to_string()
    } else if role.contains("UI") || role.contains("UX") || role.contains("设计") {
        "独立 UI/UX 角色暂不生成；frontend 负责组件库页面、基础交互和视觉一致性，architect 负责体验优先级与范围取舍。".to_string()
    } else if role.contains("快速原型") || role.to_ascii_lowercase().contains("rapid-prototyper")
    {
        "快速原型职责并入 architect 与 frontend：architect 负责需求优先级和范围控制，frontend 通过组件库快速搭建验证路径。".to_string()
    } else if role.contains("安全") || role.to_ascii_lowercase().contains("security") {
        "独立安全工程师暂不生成；compliance 负责安全/合规边界，backend 负责认证授权和输入校验实现，后续高风险阶段再拆分独立安全角色。".to_string()
    } else {
        "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色。"
            .to_string()
    }
}

fn omitted_rationale_for_unenabled_role(solution: &AgentSolution, role: &str) -> String {
    let canonical_role = canonical_role_from_label(role);
    if let Some((_, rationale)) = solution
        .role_rationale
        .iter()
        .find(|(original_role, _)| role_label_matches_canonical(original_role, &canonical_role))
    {
        if role_rationale_marks_unactivated_optional(rationale) {
            return format!(
                "{}；当前未激活，未生成独立执行 Agent。",
                rationale.trim().trim_end_matches('。')
            );
        }
    }

    default_omitted_package_role_rationale(role)
}

fn infer_enabled_roles(
    solution: &AgentSolution,
    solution_mode: &str,
    delivery_mode: Option<&str>,
) -> Vec<String> {
    let mut roles = vec![
        "frontend".to_string(),
        "backend".to_string(),
        "docs".to_string(),
        "reviewer".to_string(),
    ];
    let unactivated_optional_roles = unactivated_optional_canonical_roles(solution);
    let team_text = solution
        .team_composition
        .iter()
        .filter(|role| {
            !unactivated_optional_roles
                .iter()
                .any(|optional| role_label_matches_canonical(role, optional))
        })
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    let rationale_text = solution
        .role_rationale
        .iter()
        .filter(|(role, rationale)| {
            !role_rationale_marks_unactivated_optional(rationale)
                && !unactivated_optional_roles
                    .iter()
                    .any(|optional| role_label_matches_canonical(role, optional))
        })
        .map(|(role, rationale)| format!("{role} {rationale}"))
        .collect::<Vec<_>>()
        .join(" ");
    let role_hint_text = format!(
        "{} {} {}",
        team_text, solution.architecture_summary, rationale_text
    );
    let lower = role_hint_text.to_lowercase();

    if lower.contains("架构") || solution_mode != "fast-mvp" {
        roles.push("architect".to_string());
    }
    if lower.contains("测试") || lower.contains("qa") || lower.contains("ux") {
        roles.push("qa".to_string());
    }
    if lower.contains("数据库") || lower.contains("data") {
        roles.push("database".to_string());
    }
    if lower.contains("devops") || lower.contains("ci/cd") || lower.contains("sre") {
        roles.push("devops".to_string());
    }
    if lower.contains("安全") || lower.contains("oauth") || lower.contains("合规") {
        roles.push("compliance".to_string());
    }
    if lower.contains("小程序")
        || lower.contains("wechat")
        || lower.contains("mini program")
        || lower.contains("miniapp")
        || delivery_mode == Some("web-miniapp")
    {
        roles.push("miniapp".to_string());
        roles.push("qa".to_string());
    }

    roles.sort();
    roles.dedup();
    roles
}

fn unactivated_optional_canonical_roles(solution: &AgentSolution) -> Vec<String> {
    let mut roles = Vec::new();
    for (role, rationale) in &solution.role_rationale {
        if role_rationale_marks_unactivated_optional(rationale) {
            roles.push(canonical_role_from_label(role));
        }
    }
    roles.sort();
    roles.dedup();
    roles
}

fn role_rationale_marks_unactivated_optional(rationale: &str) -> bool {
    let lower = rationale.to_lowercase();
    let says_optional = rationale.contains("预留")
        || rationale.contains("可选")
        || rationale.contains("待确认")
        || rationale.contains("若")
        || rationale.contains("如果")
        || lower.contains("optional");
    let says_not_active = rationale.contains("当前阶段为可选")
        || rationale.contains("待确认后启用")
        || rationale.contains("未启用")
        || rationale.contains("未激活")
        || rationale.contains("后续再启用")
        || rationale.contains("未来启用")
        || lower.contains("not active");

    says_optional && says_not_active
}

fn infer_later_roles(solution: &AgentSolution) -> Vec<String> {
    let mut roles = Vec::new();
    let lower = solution.team_composition.join(" ").to_lowercase();
    if !lower.contains("测试") {
        roles.push("qa".to_string());
    }
    if !lower.contains("docs") {
        roles.push("docs".to_string());
    }
    roles
}

fn infer_delivery_mode(session: &AgentSession) -> String {
    let text = format!(
        "{} {} {} {}",
        session.readiness.product_type.clone().unwrap_or_default(),
        session.understanding_summary.clone().unwrap_or_default(),
        session.readiness.key_features.join(" "),
        session.readiness.constraints.join(" ")
    )
    .to_lowercase();

    if (text.contains("web") || text.contains("网页") || text.contains("后台"))
        && (text.contains("小程序")
            || text.contains("wechat")
            || text.contains("mini program")
            || text.contains("miniapp"))
    {
        "web-miniapp".to_string()
    } else if text.contains("landing") || text.contains("落地页") {
        "landing-page".to_string()
    } else if text.contains("showcase") || text.contains("展示") {
        "showcase-site".to_string()
    } else if text.contains("mcp") || text.contains("skill") || text.contains("内部") {
        "internal-tool".to_string()
    } else if text.contains("saas") || text.contains("平台") {
        "saas-platform".to_string()
    } else {
        "web-app".to_string()
    }
}

fn infer_project_type(session: &AgentSession) -> String {
    session
        .readiness
        .product_type
        .clone()
        .unwrap_or_else(|| "web-app".to_string())
}

fn infer_primary_workstream(solution: &AgentSolution) -> String {
    let lower = solution.title.to_lowercase();
    if lower.contains("微服务") || lower.contains("后端") {
        "backend".to_string()
    } else if lower.contains("网站") || lower.contains("web") || lower.contains("前端") {
        "frontend".to_string()
    } else {
        "fullstack".to_string()
    }
}

fn parse_bootstrap_result(raw: &str, workspace_path: &str) -> Result<AgentBootstrapResult, String> {
    let value: Value =
        serde_json::from_str(raw).map_err(|_| "desktop_bootstrap_response_invalid".to_string())?;
    let stage = value
        .get("Stage")
        .or_else(|| value.get("stage"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let message = value
        .get("Message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .unwrap_or("初始化流程返回了未知结果。")
        .to_string();
    let handoff_path = value
        .get("HandoffPath")
        .or_else(|| value.get("handoffPath"))
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let generated_files = value
        .get("GeneratedFiles")
        .or_else(|| value.get("generatedFiles"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let postcheck_passed = value
        .get("Postcheck")
        .or_else(|| value.get("postcheck"))
        .and_then(|postcheck| {
            postcheck
                .get("Passed")
                .or_else(|| postcheck.get("passed"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(stage == "implementation_ready");

    let success =
        stage == "implementation_ready" && postcheck_passed && !generated_files.is_empty();

    Ok(AgentBootstrapResult {
        status: if success {
            "success".to_string()
        } else {
            "failure".to_string()
        },
        workspace_path: workspace_path.to_string(),
        generated_files,
        handoff_path,
        postcheck_passed,
        user_facing_message: message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commonhe_bridge::provider::{self, ProviderConfig};
    use std::collections::VecDeque;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};
    use std::sync::Arc;
    use std::thread;

    fn repo_tmp_dir(name: &str) -> PathBuf {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repo root should exist")
            .to_path_buf();
        let dir = repo_root
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!("{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(3)
            .expect("repo root should exist")
            .to_path_buf()
    }

    fn with_temp_codex_home<T>(
        auth_json: Option<&str>,
        config_toml: Option<&str>,
        action: impl FnOnce() -> T,
    ) -> T {
        let _guard = crate::commonhe_bridge::test_env_lock();
        let temp = repo_tmp_dir("agent-temp-codex-home");
        let codex_home = temp.join("codex-home");
        fs::create_dir_all(&codex_home).expect("codex home should be created");
        if let Some(auth_json) = auth_json {
            fs::write(codex_home.join("auth.json"), auth_json)
                .expect("auth json should be written");
        }
        if let Some(config_toml) = config_toml {
            fs::write(codex_home.join("config.toml"), config_toml)
                .expect("config toml should be written");
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
        let _ = fs::remove_dir_all(&temp);
        result
    }

    fn with_commonhe_runtime_root<T>(runtime_root: &Path, action: impl FnOnce() -> T) -> T {
        let _guard = crate::commonhe_bridge::test_env_lock();
        let previous_runtime_root = std::env::var_os("COMMONHE_RUNTIME_ROOT");
        unsafe {
            std::env::set_var("COMMONHE_RUNTIME_ROOT", runtime_root);
        }

        let result = catch_unwind(AssertUnwindSafe(action));

        match previous_runtime_root {
            Some(previous) => unsafe {
                std::env::set_var("COMMONHE_RUNTIME_ROOT", previous);
            },
            None => unsafe {
                std::env::remove_var("COMMONHE_RUNTIME_ROOT");
            },
        }

        match result {
            Ok(value) => value,
            Err(payload) => resume_unwind(payload),
        }
    }

    fn spawn_agent_endpoint_capture_server(paths: Arc<TestMutex<Vec<String>>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("agent endpoint server should bind");
        let addr = listener
            .local_addr()
            .expect("agent endpoint server should expose addr");
        thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                paths
                    .lock()
                    .expect("path capture lock should not be poisoned")
                    .push(path.clone());

                let (status, body) = if path.contains("/responses") {
                    (
                        "200 OK",
                        json!({
                            "output": [{
                                "type": "message",
                                "content": [{
                                    "type": "output_text",
                                    "text": "{\"mode\":\"question\",\"assistantMessage\":\"继续澄清\",\"understandingSummary\":\"测试\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"
                                }]
                            }]
                        })
                        .to_string(),
                    )
                } else {
                    (
                        "500 Internal Server Error",
                        r#"{"error":{"message":"chat endpoint should not be used"}}"#.to_string(),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_endpoint_capture_server(paths: Arc<TestMutex<Vec<String>>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("chat endpoint server should bind");
        let addr = listener
            .local_addr()
            .expect("chat endpoint server should expose addr");
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0u8; 8192];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]);
            let path = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .unwrap_or("/")
                .to_string();
            paths
                .lock()
                .expect("path capture lock should not be poisoned")
                .push(path.clone());

            let (status, body) = if path.contains("/chat/completions") {
                (
                    "200 OK",
                    r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"继续澄清\",\"understandingSummary\":\"测试\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"}}]}"#,
                )
            } else {
                (
                    "500 Internal Server Error",
                    r#"{"error":{"message":"responses endpoint should not be used"}}"#,
                )
            };
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.as_bytes().len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_missing_content_then_success_server(
        requests: Arc<TestMutex<Vec<String>>>,
    ) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("chat retry server should bind");
        let addr = listener
            .local_addr()
            .expect("chat retry server should expose addr");
        thread::spawn(move || {
            for index in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request.clone());

                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if !path.contains("/chat/completions") {
                    (
                        "500 Internal Server Error",
                        r#"{"error":{"message":"unexpected endpoint"}}"#,
                    )
                } else if index == 0 {
                    ("200 OK", r#"{"choices":[{"message":{"content":""}}]}"#)
                } else {
                    (
                        "200 OK",
                        r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"电商网站需要先确认目标用户和核心交易流程。\",\"understandingSummary\":\"用户要做电商网站\",\"readiness\":{\"productType\":\"电商网站\",\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\",\"核心交易流程\"],\"readyForSolutions\":false}}"}}]}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_empty_body_then_success_server(requests: Arc<TestMutex<Vec<String>>>) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("chat empty-body retry server should bind");
        let addr = listener
            .local_addr()
            .expect("chat empty-body retry server should expose addr");
        thread::spawn(move || {
            for index in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request.clone());

                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if !path.contains("/chat/completions") {
                    (
                        "500 Internal Server Error",
                        r#"{"error":{"message":"unexpected endpoint"}}"#,
                    )
                } else if index == 0 {
                    ("200 OK", "")
                } else {
                    (
                        "200 OK",
                        r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"空响应后已恢复。\",\"understandingSummary\":\"用户要做学生管理系统\",\"readiness\":{\"productType\":\"学生管理系统\",\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"}}]}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_two_empty_bodies_then_success_server(
        requests: Arc<TestMutex<Vec<String>>>,
    ) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("chat two-empty-body retry server should bind");
        let addr = listener
            .local_addr()
            .expect("chat two-empty-body retry server should expose addr");
        thread::spawn(move || {
            for index in 0..3 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request);

                let body = if index < 2 {
                    ""
                } else {
                    r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"连续空响应后已恢复。\",\"understandingSummary\":\"用户要做学生管理系统\",\"readiness\":{\"productType\":\"学生管理系统\",\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"}}]}"#
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_always_empty_body_server(requests: Arc<TestMutex<Vec<String>>>) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("chat empty-body server should bind");
        let addr = listener
            .local_addr()
            .expect("chat empty-body server should expose addr");
        thread::spawn(move || {
            for _ in 0..3 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(String::from_utf8_lossy(&buffer[..read]).to_string());
                let response = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_headers_then_delayed_body_server() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("delayed-body server should bind");
        let addr = listener
            .local_addr()
            .expect("delayed-body server should expose addr");
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer);
            let headers = "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 64\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(headers.as_bytes());
            thread::sleep(Duration::from_millis(300));
            let _ = stream.write_all(br#"{"choices":[{"message":{"content":"late"}}]}"#);
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_plain_text_until_repair_success_server(
        requests: Arc<TestMutex<Vec<String>>>,
    ) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("chat contract repair server should bind");
        let addr = listener
            .local_addr()
            .expect("chat contract repair server should expose addr");
        thread::spawn(move || {
            for index in 0..7 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request.clone());

                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if !path.contains("/chat/completions") {
                    (
                        "500 Internal Server Error",
                        r#"{"error":{"message":"unexpected endpoint"}}"#,
                    )
                } else if index < 6 {
                    (
                        "200 OK",
                        r#"{"choices":[{"message":{"content":"我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？"}}]}"#,
                    )
                } else {
                    (
                        "200 OK",
                        r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？\",\"understandingSummary\":\"用户想做网站，但目标用户仍需确认。\",\"readiness\":{\"productType\":\"网站\",\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"}}]}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn valid_three_solutions_chat_body() -> String {
        json!({
            "choices": [{
                "message": {
                    "content": json!({
                        "mode": "solutions",
                        "assistantMessage": "已根据确认生成三套方案。",
                        "understandingSummary": "已确认需求。",
                        "readiness": {
                            "productType": "MCP",
                            "targetUsers": "企业员工",
                            "coreProblem": "移动端使用受限",
                            "keyFeatures": ["登录", "聊天", "知识库"],
                            "constraints": ["Android 8+"],
                            "summaryPresented": true,
                            "summaryConfirmed": true,
                            "missingFields": [],
                            "readyForSolutions": true
                        },
                        "solutions": [
                            {
                                "id": "A",
                                "title": "方案 A",
                                "architectureSummary": "架构 A",
                                "teamComposition": ["产品经理", "前端开发者"],
                                "tokenEstimate": "10k",
                                "recommendationText": "推荐 A",
                                "roleRationale": {"产品经理": "负责范围", "前端开发者": "负责实现"},
                                "omittedRoleRationale": {"QA 测试员": "后续加入"}
                            },
                            {
                                "id": "B",
                                "title": "方案 B",
                                "architectureSummary": "架构 B",
                                "teamComposition": ["产品经理", "后端架构师"],
                                "tokenEstimate": "12k",
                                "recommendationText": "推荐 B",
                                "roleRationale": {"产品经理": "负责范围", "后端架构师": "负责接口"},
                                "omittedRoleRationale": {"增长黑客": "当前不需要"}
                            },
                            {
                                "id": "C",
                                "title": "方案 C",
                                "architectureSummary": "架构 C",
                                "teamComposition": ["产品经理", "移动应用开发者"],
                                "tokenEstimate": "14k",
                                "recommendationText": "推荐 C",
                                "roleRationale": {"产品经理": "负责范围", "移动应用开发者": "负责 Android"},
                                "omittedRoleRationale": {"安全工程师": "后续专项处理"}
                            }
                        ]
                    }).to_string()
                }
            }]
        })
        .to_string()
    }

    fn spawn_confirmed_summary_then_solutions_server(
        requests: Arc<TestMutex<Vec<String>>>,
    ) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("confirmed solutions server should bind");
        let addr = listener
            .local_addr()
            .expect("confirmed solutions server should expose addr");
        thread::spawn(move || {
            for index in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request.clone());

                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if !path.contains("/chat/completions") {
                    (
                        "500 Internal Server Error",
                        r#"{"error":{"message":"unexpected endpoint"}}"#.to_string(),
                    )
                } else if index == 0 {
                    (
                        "200 OK",
                        json!({
                            "choices": [{
                                "message": {
                                    "content": "完美，信息已经很充分了。让我总结确认，然后给出方案。\n\n**理解确认：**\n\n| 维度 | 内容 |\n|---|---|\n| 产品 | MengBuildAI 安卓版 |\n\n确认无误。我将基于 Capacitor 混合方案，输出三套差异化的实施方案，供你选择。"
                                }
                            }]
                        })
                        .to_string(),
                    )
                } else {
                    ("200 OK", valid_three_solutions_chat_body())
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_prefaced_fenced_solutions_then_raw_solutions_server(
        requests: Arc<TestMutex<Vec<String>>>,
    ) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("prefaced fenced solutions server should bind");
        let addr = listener
            .local_addr()
            .expect("prefaced fenced solutions server should expose addr");
        thread::spawn(move || {
            for index in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                requests
                    .lock()
                    .expect("request capture lock should not be poisoned")
                    .push(request);

                let body = if index == 0 {
                    let decision = json!({
                        "mode": "solutions",
                        "assistantMessage": "已生成三套方案。",
                        "readiness": {
                            "productType": "Android App",
                            "targetUsers": "企业员工",
                            "coreProblem": "移动端使用受限",
                            "keyFeatures": ["登录", "聊天", "知识库"],
                            "constraints": ["Kotlin", "Compose"],
                            "summaryPresented": true,
                            "summaryConfirmed": true,
                            "missingFields": [],
                            "readyForSolutions": true
                        },
                        "solutions": [
                            {
                                "id": "A",
                                "title": "轻量级双人核心团队",
                                "architectureSummary": "架构 A",
                                "teamComposition": ["前端开发者"],
                                "tokenEstimate": "10k",
                                "recommendationText": "推荐 A",
                                "roleRationale": {"前端开发者": "负责实现"},
                                "omittedRoleRationale": {"QA 测试员": "后续加入"}
                            },
                            {
                                "id": "B",
                                "title": "标准四人团队",
                                "architectureSummary": "架构 B",
                                "teamComposition": ["后端架构师"],
                                "tokenEstimate": "12k",
                                "recommendationText": "推荐 B",
                                "roleRationale": {"后端架构师": "负责接口"},
                                "omittedRoleRationale": {"增长黑客": "当前不需要"}
                            },
                            {
                                "id": "C",
                                "title": "完整七人团队",
                                "architectureSummary": "架构 C",
                                "teamComposition": ["安全工程师"],
                                "tokenEstimate": "14k",
                                "recommendationText": "推荐 C",
                                "roleRationale": {"安全工程师": "负责安全"},
                                "omittedRoleRationale": {"DevOps 自动化师": "当前不需要"}
                            }
                        ]
                    })
                    .to_string();
                    json!({
                        "choices": [{
                            "message": {
                                "content": format!("你说得对，我绕远了。现在直接重新输出干净版本：\n\n```json\n{decision}\n```")
                            }
                        }]
                    })
                    .to_string()
                } else {
                    valid_three_solutions_chat_body()
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_chat_sse_response_server(requests: Arc<TestMutex<Vec<String>>>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").expect("chat sse server should bind");
        let addr = listener
            .local_addr()
            .expect("chat sse server should expose addr");
        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0u8; 8192];
            let read = stream.read(&mut buffer).unwrap_or(0);
            let request = String::from_utf8_lossy(&buffer[..read]).to_string();
            requests
                .lock()
                .expect("request capture lock should not be poisoned")
                .push(request.clone());

            let body = "data: {\"choices\":[{\"message\":{\"content\":\"{\\\"mode\\\":\\\"question\\\",\\\"assistantMessage\\\":\\\"继续确认双端范围\\\",\\\"understandingSummary\\\":\\\"电商网站\\\",\\\"readiness\\\":{\\\"productType\\\":\\\"电商网站\\\",\\\"keyFeatures\\\":[],\\\"constraints\\\":[],\\\"summaryPresented\\\":false,\\\"summaryConfirmed\\\":false,\\\"missingFields\\\":[\\\"技术约束\\\"],\\\"readyForSolutions\\\":false}}\"}}]}\n\ndata: [DONE]\n";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.as_bytes().len()
            );
            let _ = stream.write_all(response.as_bytes());
        });
        format!("http://{}", addr)
    }

    fn spawn_semantic_fallback_server() -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("semantic fallback server should bind");
        let addr = listener
            .local_addr()
            .expect("semantic fallback server should have addr");
        thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/");
                let (status, body) = if path.contains("/responses") {
                    (
                        "404 Not Found",
                        r#"{"error":{"message":"responses unavailable"}}"#.to_string(),
                    )
                } else {
                    (
                        "200 OK",
                        json!({
                            "choices": [{
                                "message": {
                                    "content": "{\"passed\":true,\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"chat fallback ok\",\"confidence\":\"high\",\"reviewerAgent\":\"星梦梦\"}"
                                }
                            }]
                        })
                        .to_string(),
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn spawn_responses_failure_capture_server(paths: Arc<TestMutex<Vec<String>>>) -> String {
        let listener =
            TcpListener::bind("127.0.0.1:0").expect("responses failure server should bind");
        let addr = listener
            .local_addr()
            .expect("responses failure server should expose addr");
        thread::spawn(move || {
            for _ in 0..2 {
                let Ok((mut stream, _)) = listener.accept() else {
                    return;
                };
                let mut buffer = [0u8; 8192];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                let path = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("/")
                    .to_string();
                paths
                    .lock()
                    .expect("path capture lock should not be poisoned")
                    .push(path.clone());

                let (status, body) = if path.contains("/responses") {
                    (
                        "502 Bad Gateway",
                        r#"{"error":{"message":"responses temporarily unavailable"}}"#,
                    )
                } else {
                    (
                        "200 OK",
                        r#"{"choices":[{"message":{"content":"{\"mode\":\"question\",\"assistantMessage\":\"chat fallback should not run\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"}}]}"#,
                    )
                };
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.as_bytes().len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        format!("http://{}", addr)
    }

    fn base_session() -> AgentSession {
        AgentSession {
            session_id: "test-session".to_string(),
            provider: "deepseek".to_string(),
            model: "deepseek-v4-flash".to_string(),
            api_key: "test".to_string(),
            base_url: "https://api.deepseek.com/v1".to_string(),
            wire_api: "chat_completions".to_string(),
            workspace_path: "E:/Projects/Demo".to_string(),
            system_prompt: "system".to_string(),
            orchestrator_path: PathBuf::from(
                "E:/WorkSoft/CommonHE/tools/common-he-init-orchestrator.ps1",
            ),
            messages: vec![],
            understanding_summary: None,
            readiness: AgentReadiness::default(),
            solutions: vec![],
            tool_calls: vec![],
            bootstrap_result: None,
            selected_solution_id: None,
            project_name: Some("Demo协作包".to_string()),
            target_client: Some(TargetClient::Codex),
            selected_capabilities: vec![],
            semantic_review_status: "not_started".to_string(),
            semantic_review_issues: vec![],
            dialogue_round_count: 0,
            finished: false,
        }
    }

    fn test_role_rationale() -> HashMap<String, String> {
        HashMap::from([
            (
                "frontend".to_string(),
                "用户可见 Web 端需要前端实现。".to_string(),
            ),
            (
                "backend".to_string(),
                "核心业务和 API 需要后端承接。".to_string(),
            ),
        ])
    }

    fn test_omitted_role_rationale() -> HashMap<String, String> {
        HashMap::from([(
            "qa".to_string(),
            "当前测试范围较小，先由 reviewer 做首轮验证；星梦梦可继续追问。".to_string(),
        )])
    }

    fn final_package_failure_review() -> SemanticReviewResult {
        SemanticReviewResult {
            passed: false,
            blocking_issues: vec!["最终协作包没有保留用户原始需求。".to_string()],
            questions_for_meng_xingxing: vec!["请解释最终产物为何缺少用户原始需求。".to_string()],
            required_repairs: vec!["修正生成文件后重新交给星梦梦验收。".to_string()],
            review_summary: "星梦梦最终验收未通过。".to_string(),
            confidence: "high".to_string(),
        }
    }

    struct FakeSemanticRuntime {
        reviews: TestMutex<VecDeque<SemanticReviewResult>>,
    }

    impl FakeSemanticRuntime {
        fn new(reviews: Vec<SemanticReviewResult>) -> Self {
            Self {
                reviews: TestMutex::new(VecDeque::from(reviews)),
            }
        }

        fn passing() -> Self {
            Self::new(vec![
                SemanticReviewResult {
                    passed: true,
                    blocking_issues: vec![],
                    questions_for_meng_xingxing: vec![],
                    required_repairs: vec![],
                    review_summary: "星梦梦方案预审通过。".to_string(),
                    confidence: "high".to_string(),
                },
                SemanticReviewResult {
                    passed: true,
                    blocking_issues: vec![],
                    questions_for_meng_xingxing: vec![],
                    required_repairs: vec![],
                    review_summary: "星梦梦最终验收通过。".to_string(),
                    confidence: "high".to_string(),
                },
            ])
        }
    }

    impl SemanticAgentRuntime for FakeSemanticRuntime {
        fn review(
            &self,
            _session: &AgentSession,
            _round: usize,
            _context: &SemanticReviewContext,
        ) -> Result<SemanticReviewResult, String> {
            self.reviews
                .lock()
                .expect("fake runtime lock should not be poisoned")
                .pop_front()
                .ok_or_else(|| "fake_semantic_review_exhausted".to_string())
        }

        fn repair(
            &self,
            session: &AgentSession,
            review: &SemanticReviewResult,
            round: usize,
        ) -> Result<RepairDecision, String> {
            let selected_solution_id = session
                .selected_solution_id
                .as_ref()
                .ok_or_else(|| "fake_repair_missing_selected_solution".to_string())?;
            let mut updated_solution = session
                .solutions
                .iter()
                .find(|solution| &solution.id == selected_solution_id)
                .cloned()
                .ok_or_else(|| "fake_repair_selected_solution_not_found".to_string())?;
            for role in ["qa", "reviewer"] {
                if !updated_solution
                    .team_composition
                    .iter()
                    .any(|item| item == role)
                {
                    updated_solution.team_composition.push(role.to_string());
                }
                updated_solution.role_rationale.insert(
                    role.to_string(),
                    format!("星梦梦提出阻断后，梦星星确认 {role} 是双端交付验收必须角色。"),
                );
                updated_solution.omitted_role_rationale.remove(role);
            }

            Ok(RepairDecision {
                round,
                status: "repaired".to_string(),
                issues: review.blocking_issues.clone(),
                response_summary: "梦星星已接受星梦梦阻断项并补齐角色取舍。".to_string(),
                updated_solution: Some(updated_solution),
            })
        }
    }

    #[test]
    fn session_api_key_resolution_uses_local_codex_auth_when_input_is_empty() {
        let _guard = crate::commonhe_bridge::test_env_lock();
        let previous_openai = std::env::var("OPENAI_API_KEY").ok();
        let previous_codex = std::env::var("CODEX_API_KEY").ok();

        unsafe {
            std::env::set_var("OPENAI_API_KEY", "test-api-key-from-env");
            std::env::remove_var("CODEX_API_KEY");
        }

        let resolved = resolve_session_api_key("codex", "");

        match previous_openai {
            Some(value) => unsafe { std::env::set_var("OPENAI_API_KEY", value) },
            None => unsafe { std::env::remove_var("OPENAI_API_KEY") },
        }
        match previous_codex {
            Some(value) => unsafe { std::env::set_var("CODEX_API_KEY", value) },
            None => unsafe { std::env::remove_var("CODEX_API_KEY") },
        }

        assert_eq!(
            resolved.expect("session should resolve local auth"),
            "test-api-key-from-env"
        );
    }

    #[test]
    fn solutions_ready_snapshot_must_expose_solution_selector_tool_request() {
        let mut session = base_session();
        session.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: "目前理解为：产品形态是网站；目标用户是学生；核心问题是高效完成选课；关键功能包括选课、排课；约束条件是Web。这样理解对吗？如果准确，我就继续整理三套方案。".to_string(),
            ..Default::default()
        });
        session.messages.push(AgentMessage {
            role: "user".to_string(),
            content: "是的，按这个理解。".to_string(),
            ..Default::default()
        });
        apply_agent_decision(
            &mut session,
            AgentDecision {
                mode: "solutions".to_string(),
                assistant_message: "这里是三个方案".to_string(),
                understanding_summary: Some("目标已经澄清".to_string()),
                readiness: Some(AgentReadiness {
                    product_type: Some("网站".to_string()),
                    target_users: Some("学生".to_string()),
                    core_problem: Some("高效完成选课".to_string()),
                    key_features: vec!["选课".to_string(), "排课".to_string()],
                    constraints: vec!["Web".to_string()],
                    summary_presented: true,
                    summary_confirmed: true,
                    missing_fields: vec![],
                    ready_for_solutions: true,
                }),
                solutions: Some(vec![
                    AgentSolution {
                        id: "A".to_string(),
                        title: "方案A".to_string(),
                        architecture_summary: "架构A".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "低".to_string(),
                        recommendation_text: "推荐A".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "B".to_string(),
                        title: "方案B".to_string(),
                        architecture_summary: "架构B".to_string(),
                        team_composition: vec!["pm".to_string(), "engineer".to_string()],
                        token_estimate: "中".to_string(),
                        recommendation_text: "推荐B".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "C".to_string(),
                        title: "方案C".to_string(),
                        architecture_summary: "架构C".to_string(),
                        team_composition: vec!["pm".to_string(), "qa".to_string()],
                        token_estimate: "高".to_string(),
                        recommendation_text: "推荐C".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                ]),
            },
        )
        .expect("decision should apply");

        let snapshot = snapshot_from_session("session-1", &session);

        assert_eq!(snapshot.stage, "solutions_ready");
        assert_eq!(snapshot.tool_calls.len(), 1);
        assert_eq!(snapshot.tool_calls[0].tool_name, "open_solution_selector");
        assert_eq!(snapshot.tool_calls[0].status, "requested");
    }

    #[test]
    fn solutions_are_blocked_until_readiness_is_confirmed() {
        let mut session = base_session();
        apply_agent_decision(
            &mut session,
            AgentDecision {
                mode: "solutions".to_string(),
                assistant_message: "这里是三个方案".to_string(),
                understanding_summary: Some("我理解你要做一个网站".to_string()),
                readiness: Some(AgentReadiness {
                    product_type: Some("网站".to_string()),
                    target_users: Some("学生".to_string()),
                    core_problem: Some("高效完成选课".to_string()),
                    key_features: vec!["选课".to_string(), "排课".to_string()],
                    constraints: vec!["Web".to_string()],
                    summary_presented: true,
                    summary_confirmed: false,
                    missing_fields: vec!["用户确认".to_string()],
                    ready_for_solutions: false,
                }),
                solutions: Some(vec![
                    AgentSolution {
                        id: "A".to_string(),
                        title: "方案A".to_string(),
                        architecture_summary: "架构A".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "低".to_string(),
                        recommendation_text: "推荐A".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "B".to_string(),
                        title: "方案B".to_string(),
                        architecture_summary: "架构B".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "中".to_string(),
                        recommendation_text: "推荐B".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "C".to_string(),
                        title: "方案C".to_string(),
                        architecture_summary: "架构C".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "高".to_string(),
                        recommendation_text: "推荐C".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                ]),
            },
        )
        .expect("decision should apply");

        assert!(session.solutions.is_empty());
        assert_eq!(session.tool_calls.len(), 0);
        assert!(session
            .messages
            .last()
            .expect("assistant follow-up should exist")
            .content
            .contains("这样理解对吗"));
    }

    #[test]
    fn model_cannot_self_confirm_solutions_without_user_confirmation() {
        let mut session = base_session();
        session.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: "目前理解为：产品形态是网站；目标用户是学生；核心问题是高效完成选课；关键功能包括选课、排课；约束条件是Web。这样理解对吗？如果准确，我就继续整理三套方案。".to_string(),
            ..Default::default()
        });

        apply_agent_decision(
            &mut session,
            AgentDecision {
                mode: "solutions".to_string(),
                assistant_message: "这里是三个方案".to_string(),
                understanding_summary: Some("我理解你要做一个网站".to_string()),
                readiness: Some(AgentReadiness {
                    product_type: Some("网站".to_string()),
                    target_users: Some("学生".to_string()),
                    core_problem: Some("高效完成选课".to_string()),
                    key_features: vec!["选课".to_string(), "排课".to_string()],
                    constraints: vec!["Web".to_string()],
                    summary_presented: true,
                    summary_confirmed: true,
                    missing_fields: vec![],
                    ready_for_solutions: true,
                }),
                solutions: Some(vec![
                    AgentSolution {
                        id: "A".to_string(),
                        title: "方案A".to_string(),
                        architecture_summary: "架构A".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "低".to_string(),
                        recommendation_text: "推荐A".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "B".to_string(),
                        title: "方案B".to_string(),
                        architecture_summary: "架构B".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "中".to_string(),
                        recommendation_text: "推荐B".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                    AgentSolution {
                        id: "C".to_string(),
                        title: "方案C".to_string(),
                        architecture_summary: "架构C".to_string(),
                        team_composition: vec!["pm".to_string()],
                        token_estimate: "高".to_string(),
                        recommendation_text: "推荐C".to_string(),
                        role_rationale: test_role_rationale(),
                        omitted_role_rationale: test_omitted_role_rationale(),
                    },
                ]),
            },
        )
        .expect("decision should apply");

        assert!(!session.readiness.summary_confirmed);
        assert!(session.solutions.is_empty());
        assert!(session
            .messages
            .last()
            .expect("assistant follow-up should exist")
            .content
            .contains("这样理解对吗"));
    }

    #[test]
    fn short_confirmation_only_counts_after_summary_is_presented() {
        let readiness_without_summary = infer_readiness_from_messages(
            &[AgentMessage {
                role: "user".to_string(),
                content: "可以".to_string(),
                ..Default::default()
            }],
            None,
        );
        assert!(!readiness_without_summary.summary_confirmed);

        let readiness_with_summary = infer_readiness_from_messages(
            &[
                AgentMessage {
                    role: "assistant".to_string(),
                    content:
                        "目前理解为：产品形态是网站。这样理解对吗？如果准确，我就继续整理三套方案。"
                            .to_string(),
                    ..Default::default()
                },
                AgentMessage {
                    role: "user".to_string(),
                    content: "可以".to_string(),
                    ..Default::default()
                },
            ],
            None,
        );
        assert!(readiness_with_summary.summary_confirmed);
    }

    #[test]
    fn natural_confirmation_sentence_counts_after_summary_is_presented() {
        let readiness_with_summary = infer_readiness_from_messages(
            &[
                AgentMessage {
                    role: "assistant".to_string(),
                    content:
                        "目前理解为：产品形态是网站。这样理解对吗？如果准确，我就继续整理三套方案。"
                            .to_string(),
                    ..Default::default()
                },
                AgentMessage {
                    role: "user".to_string(),
                    content: "是的，按这个理解，请给我三个方案。".to_string(),
                    ..Default::default()
                },
            ],
            None,
        );
        assert!(readiness_with_summary.summary_confirmed);
    }

    #[test]
    fn compact_confirmation_variants_count_after_summary_is_presented() {
        for reply in ["准确。", "确定", "确认准确", "没问题"] {
            let readiness_without_summary = infer_readiness_from_messages(
                &[AgentMessage {
                    role: "user".to_string(),
                    content: reply.to_string(),
                    ..Default::default()
                }],
                None,
            );
            assert!(
                !readiness_without_summary.summary_confirmed,
                "{reply} must not self-confirm before the assistant presents a summary"
            );

            let readiness_with_summary = infer_readiness_from_messages(
                &[
                    AgentMessage {
                        role: "assistant".to_string(),
                        content:
                            "目前理解为：产品形态是网站。这样理解对吗？如果准确，我就继续整理三套方案。"
                                .to_string(),
                        ..Default::default()
                    },
                    AgentMessage {
                        role: "user".to_string(),
                        content: reply.to_string(),
                        ..Default::default()
                    },
                ],
                None,
            );
            assert!(
                readiness_with_summary.summary_confirmed,
                "{reply} should confirm after the assistant presents a summary"
            );
        }
    }

    #[test]
    fn compact_confirmation_variants_are_provider_neutral_after_summary_is_presented() {
        let current = AgentReadiness {
            product_type: Some("网站".to_string()),
            target_users: Some("B2B 批发商".to_string()),
            core_problem: Some("快速上线电商业务".to_string()),
            key_features: vec!["商品展示".to_string(), "下单支付".to_string()],
            constraints: vec!["SaaS".to_string(), "快速上线".to_string()],
            summary_presented: true,
            summary_confirmed: false,
            missing_fields: vec!["用户确认".to_string()],
            ready_for_solutions: false,
        };
        let messages = vec![AgentMessage {
            role: "user".to_string(),
            content: "确定。".to_string(),
            ..Default::default()
        }];

        for provider in ["custom", "deepseek", "openai", "codex"] {
            let standard = merge_readiness(&current, None, &messages, None, provider);
            assert!(
                standard.summary_confirmed,
                "{provider} should accept compact confirmation after summary is presented"
            );
            assert!(standard.ready_for_solutions);
            assert!(standard.missing_fields.is_empty());
        }
    }

    #[test]
    fn custom_and_deepseek_pro_agent_timeouts_are_extended_without_changing_other_standard_providers(
    ) {
        let mut session = base_session();

        for provider in ["openai", "codex"] {
            session.provider = provider.to_string();
            assert_eq!(
                agent_request_timeout(&session),
                Duration::from_secs(60),
                "{provider} should keep the existing timeout"
            );
        }

        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        assert_eq!(agent_request_timeout(&session), Duration::from_secs(60));
        session.model = "deepseek-v4-pro".to_string();
        assert_eq!(agent_request_timeout(&session), Duration::from_secs(180));

        session.provider = "custom".to_string();
        assert_eq!(agent_request_timeout(&session), Duration::from_secs(180));
    }

    #[test]
    fn parser_recovers_question_json_wrapped_in_markdown_fence() {
        let decision = parse_agent_decision_content(
            "```json\n{\"mode\":\"question\",\"assistantMessage\":\"请补充目标用户\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}\n```",
        )
        .expect("question-stage JSON fences should keep the normal chat flow usable");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "请补充目标用户");
    }

    #[test]
    fn parser_recovers_question_json_wrapped_as_string() {
        let decision = parse_agent_decision_content(
            "\"{\\\"mode\\\":\\\"question\\\",\\\"assistantMessage\\\":\\\"请补充目标用户\\\",\\\"readiness\\\":{\\\"keyFeatures\\\":[],\\\"constraints\\\":[],\\\"summaryPresented\\\":false,\\\"summaryConfirmed\\\":false,\\\"missingFields\\\":[\\\"目标用户\\\"],\\\"readyForSolutions\\\":false}}\"",
        )
        .expect("question-stage string-wrapped JSON should keep the normal chat flow usable");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "请补充目标用户");
    }

    #[test]
    fn parser_falls_back_to_plain_text_question_when_model_skips_json() {
        let decision = parse_agent_decision_content(
            "我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？",
        )
        .expect("plain text follow-up questions should keep the normal chat flow usable");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？"
        );
    }

    #[test]
    fn parser_rejects_markdown_solution_bundle_so_contract_repair_can_trigger_selector() {
        let result = parse_agent_decision_content(
            r#"## 方案A：快速验证型
### Agent团队组成
```json
{"teamComposition":["产品经理"],"roleRationale":{"产品经理":"需求管理"},"omittedRoleRationale":{"QA":"MVP阶段暂不配置"}}
```
## 方案B：标准专业型
### Agent团队组成
```json
{"teamComposition":["产品经理","前端开发者"],"roleRationale":{"前端开发者":"页面实现"},"omittedRoleRationale":{"DevOps自动化师":"暂不需要"}}
```
## 方案C：全栈专业型
### Agent团队组成
```json
{"teamComposition":["产品经理","后端架构师"],"roleRationale":{"后端架构师":"API设计"},"omittedRoleRationale":{"增长黑客":"不是MVP核心"}}
```
请您选择偏好的方案（A/B/C）。"#,
        );

        assert_eq!(result.unwrap_err(), "agent_solutions_unstructured_markdown");
        assert!(is_repairable_agent_contract_error(
            "agent_solutions_unstructured_markdown"
        ));
    }

    #[test]
    fn parser_rejects_deepseek_markdown_solution_picker_agent_bundle() {
        let result = parse_agent_decision_content(
            r#"好的，最终确认完成。现在进入方案设计环节，下面为你生成三个明确的实施方案。

---

## 方案 A：模块化全栈渐进式（推荐）

**架构摘要**：
- 单 Activity + MVI 架构

**团队组成**（从 agency-agents 中选角）：
- **后端架构师**：负责对齐后端接口清单与 SSE 流式规范
- **前端开发者**：负责 Kotlin Compose UI

**Token 预估**：约 10–14万 tokens

**不选的角色与原因**：
- **QA 测试员**：当前阶段以架构设计+指导文档为主

---

## 方案 B：平台导向分端突击式

**架构摘要**：
- 以「平台特性」为主线拆分

**团队组成**：
- **移动应用开发者**：主导所有平台特定能力实现
- **后端架构师**：负责 SSE 流式协议

**Token 预估**：约 12–16万 tokens

**不选的角色与原因**：
- **快速原型师**：本方案不以先搭 MVP 验证为目标

---

## 方案 C：接口驱动端到端压路式

**架构摘要**：
- 严格按 PRD 的子模块顺序拉通

**团队组成**：
- **后端架构师**：负责 OpenAPI 文档
- **安全工程师**：负责 Token 存储方案审计

**Token 预估**：约 8–12万 tokens

**不选的角色与原因**：
- **快速原型师**：本方案以接口契约为驱动

---

<solution-picker-agent
  instructions="StarDream 复核方案前，请从三种方案中选中推荐方案 A 并居中展示三条关键理由">
</solution-picker-agent>"#,
        );

        assert_eq!(result.unwrap_err(), "agent_solutions_unstructured_markdown");
    }

    #[test]
    fn parser_rejects_fenced_solutions_json_because_final_selector_requires_raw_json() {
        let raw = r#"```json
{
  "mode": "solutions",
  "assistantMessage": "已生成三套方案。",
  "solutions": [
    {
      "id": "A",
      "title": "方案 A",
      "architectureSummary": "架构 A",
      "teamComposition": ["产品经理"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 A",
      "roleRationale": {"产品经理": "负责范围"},
      "omittedRoleRationale": {"QA 测试员": "后续再加入"}
    },
    {
      "id": "B",
      "title": "方案 B",
      "architectureSummary": "架构 B",
      "teamComposition": ["前端开发者"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 B",
      "roleRationale": {"前端开发者": "负责实现"},
      "omittedRoleRationale": {"后端架构师": "当前不需要"}
    },
    {
      "id": "C",
      "title": "方案 C",
      "architectureSummary": "架构 C",
      "teamComposition": ["后端架构师"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 C",
      "roleRationale": {"后端架构师": "负责接口"},
      "omittedRoleRationale": {"增长黑客": "当前不需要"}
    }
  ]
}
```"#;

        let result = parse_agent_decision_content(raw);

        assert_eq!(result.unwrap_err(), "agent_solutions_unstructured_markdown");
        assert!(is_repairable_agent_contract_error(
            "agent_solutions_unstructured_markdown"
        ));
    }

    #[test]
    fn parser_rejects_prefaced_fenced_solutions_json_instead_of_plain_text_question() {
        let raw = r#"啊，我看到了，刚才输出里有冗余。让我直接重新输出干净版本：

```json
{
  "mode": "solutions",
  "assistantMessage": "已生成三套方案。",
  "solutions": [
    {
      "id": "A",
      "title": "轻量级双人核心团队",
      "architectureSummary": "架构 A",
      "teamComposition": ["前端开发者"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 A",
      "roleRationale": {"前端开发者": "负责实现"},
      "omittedRoleRationale": {"QA 测试员": "后续再加入"}
    },
    {
      "id": "B",
      "title": "标准四人团队",
      "architectureSummary": "架构 B",
      "teamComposition": ["后端架构师"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 B",
      "roleRationale": {"后端架构师": "负责接口"},
      "omittedRoleRationale": {"增长黑客": "当前不需要"}
    },
    {
      "id": "C",
      "title": "完整七人团队",
      "architectureSummary": "架构 C",
      "teamComposition": ["安全工程师"],
      "tokenEstimate": "1万 tokens",
      "recommendationText": "推荐 C",
      "roleRationale": {"安全工程师": "负责安全"},
      "omittedRoleRationale": {"DevOps 自动化师": "当前不需要"}
    }
  ]
}
```"#;

        let result = parse_agent_decision_content(raw);

        assert_eq!(result.unwrap_err(), "agent_solutions_unstructured_markdown");
    }

    #[test]
    fn parser_accepts_raw_json_three_real_solutions_and_requests_selector() {
        let raw = r#"{
  "mode": "solutions",
  "assistantMessage": "已根据你的确认生成三套可选实施方案。",
  "understandingSummary": "你要为 B2B 批发电商网站生成初始化协作包，目标用户是浴室浴霸行业中小批发商、经销商和工程采购商。",
  "readiness": {
    "productType": "网站",
    "targetUsers": "浴室浴霸行业中小批发商、经销商、工程采购商",
    "coreProblem": "快速搭建可下单、可运营、可交给 Codex 接手实施的 B2B 批发电商网站",
    "keyFeatures": ["商品展示", "下单支付", "搜索过滤", "优惠券系统", "后台管理"],
    "constraints": ["SaaS", "快速上线", "Codex 接手实施"],
    "summaryPresented": true,
    "summaryConfirmed": true,
    "missingFields": [],
    "readyForSolutions": true
  },
  "solutions": [
    {
      "id": "A",
      "title": "模块化全栈渐进式",
      "architectureSummary": "先以商品、订单、支付、后台管理四个模块拉通 MVP，再逐步补齐营销和搜索能力。",
      "teamComposition": ["产品经理", "前端开发者", "后端架构师", "UX 架构师"],
      "tokenEstimate": "10-14万 tokens",
      "recommendationText": "推荐用于快速上线并保持后续可扩展。",
      "roleRationale": {
        "产品经理": "负责范围收敛、用户故事和验收标准。",
        "前端开发者": "负责商品、购物车、订单和后台 UI 实现。",
        "后端架构师": "负责订单、支付、用户和商品接口边界。",
        "UX 架构师": "负责批发采购路径和后台信息架构。"
      },
      "omittedRoleRationale": {
        "QA 测试员": "当前是初始化协作包规划阶段，自动验收策略先由后续 Sprint 承接。"
      }
    },
    {
      "id": "B",
      "title": "营销运营优先式",
      "architectureSummary": "优先搭建商品内容、优惠券、直播带货入口和运营后台，再补齐高级采购流程。",
      "teamComposition": ["产品经理", "前端开发者", "增长黑客", "内容策略师"],
      "tokenEstimate": "12-16万 tokens",
      "recommendationText": "适合先验证获客和转化，再扩大交易能力。",
      "roleRationale": {
        "产品经理": "负责确定营销优先级和转化目标。",
        "前端开发者": "负责运营页面、活动组件和后台配置。",
        "增长黑客": "负责优惠券、直播入口和转化路径假设。",
        "内容策略师": "负责商品卖点和行业内容结构。"
      },
      "omittedRoleRationale": {
        "安全工程师": "本阶段不处理复杂风控，支付安全在后续接口实现阶段专项处理。"
      }
    },
    {
      "id": "C",
      "title": "接口契约驱动式",
      "architectureSummary": "先定义商品、订单、支付、用户和后台管理 API 契约，再按契约生成前后端实施包。",
      "teamComposition": ["产品经理", "后端架构师", "前端开发者", "安全工程师"],
      "tokenEstimate": "8-12万 tokens",
      "recommendationText": "适合后端边界明确、希望 Codex 按接口稳定推进的场景。",
      "roleRationale": {
        "产品经理": "负责按接口拆分里程碑和验收脚本。",
        "后端架构师": "负责 OpenAPI 契约、数据模型和服务边界。",
        "前端开发者": "负责根据接口契约实现页面和状态流。",
        "安全工程师": "负责支付、鉴权和日志脱敏约束。"
      },
      "omittedRoleRationale": {
        "快速原型师": "本方案以接口稳定性为主，不以快速 POC 为主。"
      }
    }
  ]
}"#;

        let decision = parse_agent_decision_content(raw)
            .expect("raw JSON with three complete solutions should pass strict parsing");

        assert_eq!(decision.mode, "solutions");
        assert_eq!(
            decision.solutions.as_ref().map(Vec::len),
            Some(3),
            "strict parser must preserve all three solution items"
        );

        let mut session = base_session();
        session.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: "目前理解为：产品形态是网站；目标用户是浴室浴霸行业批发商；核心问题是快速搭建 B2B 批发电商网站；关键功能包括商品展示、下单支付和后台管理；约束条件是 SaaS、快速上线、Codex 接手实施。这样理解对吗？如果准确，我就继续整理三套方案。".to_string(),
            ..Default::default()
        });
        session.messages.push(AgentMessage {
            role: "user".to_string(),
            content: "准确。".to_string(),
            ..Default::default()
        });

        apply_agent_decision(&mut session, decision)
            .expect("valid three-solution decision should apply");

        assert_eq!(session.solutions.len(), 3);
        assert!(session.tool_calls.iter().any(|tool_call| {
            tool_call.tool_name == "open_solution_selector" && tool_call.status == "requested"
        }));
    }

    #[test]
    fn parser_recovers_json_from_responses_output_text() {
        let decision = parse_agent_decision_response(&json!({
            "id": "resp_123",
            "object": "response",
            "output": [
                {
                    "type": "message",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "{\"mode\":\"question\",\"assistantMessage\":\"请继续补充目标用户\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"
                        }
                    ]
                }
            ]
        }))
        .expect("parser should recover nested responses output text");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "请继续补充目标用户");
    }

    #[test]
    fn parser_recovers_json_from_chat_content_array() {
        let decision = parse_agent_decision_response(&json!({
            "choices": [
                {
                    "message": {
                        "content": [
                            {
                                "type": "text",
                                "text": "{\"mode\":\"question\",\"assistantMessage\":\"请补充目标用户\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"
                            }
                        ]
                    }
                }
            ]
        }))
        .expect("parser should recover json from chat content array");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "请补充目标用户");
    }

    #[test]
    fn parser_recovers_json_from_nested_text_value() {
        let decision = parse_agent_decision_response(&json!({
            "choices": [
                {
                    "message": {
                        "content": [
                            {
                                "type": "output_text",
                                "text": {
                                    "value": "{\"mode\":\"question\",\"assistantMessage\":\"请补充约束条件\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"约束条件\"],\"readyForSolutions\":false}}"
                                }
                            }
                        ]
                    }
                }
            ]
        }))
        .expect("parser should recover json from nested text value");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "请补充约束条件");
    }

    #[test]
    fn responses_request_body_carries_instructions_and_conversation_input() {
        let mut session = base_session();
        session.provider = "codex".to_string();
        session.model = "gpt-5.4".to_string();
        session.base_url = "https://ai.xingmengmeng.com/v1".to_string();
        session.wire_api = "responses".to_string();
        session.messages = vec![
            AgentMessage {
                role: "assistant".to_string(),
                content: "请先告诉我产品形态。".to_string(),
                item_id: Some("msg_assistant_1".to_string()),
                status: Some("completed".to_string()),
            },
            AgentMessage {
                role: "user".to_string(),
                content: "我想做一个电商网站。".to_string(),
                item_id: Some("msg_user_1".to_string()),
                status: None,
            },
        ];

        let body = build_responses_request_body(&session, Some("user_message"));

        assert_eq!(body.get("model").and_then(Value::as_str), Some("gpt-5.4"));
        assert!(body.get("instructions").and_then(Value::as_str).is_some());
        let input = body
            .get("input")
            .and_then(Value::as_array)
            .expect("responses request should include input messages");
        assert_eq!(input.len(), 2);
        assert_eq!(
            input[0].get("type").and_then(Value::as_str),
            Some("message")
        );
        assert_eq!(
            input[0].get("role").and_then(Value::as_str),
            Some("assistant")
        );
        assert_eq!(
            input[0].get("status").and_then(Value::as_str),
            Some("completed")
        );
        assert!(input[0].get("id").and_then(Value::as_str).is_some());
        assert_eq!(input[1].get("role").and_then(Value::as_str), Some("user"));
        assert!(input[1].get("id").and_then(Value::as_str).is_some());
        let first_content = input[0]
            .get("content")
            .and_then(Value::as_array)
            .and_then(|items| items.first())
            .expect("responses message should include content items");
        assert_eq!(
            first_content.get("type").and_then(Value::as_str),
            Some("input_text")
        );
    }

    #[test]
    fn deepseek_chat_json_request_uses_documented_safety_parameters() {
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();

        let body = build_chat_completions_request_body(&session, Some("user_message"));

        assert_eq!(
            body.get("response_format")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str),
            Some("json_object")
        );
        assert_eq!(body.get("max_tokens").and_then(Value::as_u64), Some(4096));
        assert_eq!(
            body.get("thinking")
                .and_then(|value| value.get("type"))
                .and_then(Value::as_str),
            Some("disabled")
        );
    }

    #[test]
    fn responses_flow_falls_back_to_chat_on_bad_gateway() {
        assert!(should_fallback_from_responses_to_chat(&AgentHttpError {
            status_code: 502,
            body: "bad gateway".to_string(),
            endpoint: None,
        }));
    }

    #[test]
    fn responses_flow_does_not_fall_back_to_chat_on_transport_error() {
        assert!(!should_fallback_from_responses_to_chat(&AgentHttpError {
            status_code: 0,
            body: "error sending request for url (https://ai.xingmengmeng.com/v1/responses)"
                .to_string(),
            endpoint: None,
        }));
    }

    #[test]
    fn semantic_review_reuses_chat_fallback_when_responses_endpoint_is_unavailable() {
        let base_url = spawn_semantic_fallback_server();
        let mut session = base_session();
        let session_id = "semantic-call-log-session".to_string();
        let workspace = repo_root()
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!(
                "semantic-call-log-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
        let runtime_root = repo_root()
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!(
                "commonhe-runtime-log-root-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&runtime_root).unwrap();
        session.session_id = session_id.clone();
        session.provider = "codex".to_string();
        session.model = "gpt-5.5".to_string();
        session.base_url = base_url;
        session.wire_api = "responses".to_string();
        session.api_key = "sk-test".to_string();
        session.workspace_path = workspace.to_string_lossy().to_string();

        let value = with_commonhe_runtime_root(&runtime_root, || {
            request_semantic_json(&session, "只返回 JSON。", &json!({ "round": 1 }))
                .expect("semantic review should fall back to chat completions")
        });

        assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
        assert_eq!(
            value.get("reviewerAgent").and_then(Value::as_str),
            Some("星梦梦")
        );
        let target_call_log_path = workspace
            .join("data")
            .join("logs")
            .join("commonhe-agent-calls.jsonl");
        assert!(
            !target_call_log_path.exists(),
            "model-call log should stay in the launcher runtime root, not target workspace data/logs: {}",
            target_call_log_path.display()
        );
        let call_log_path = runtime_root
            .join("data")
            .join("logs")
            .join("commonhe-agent-calls.jsonl");
        let call_log = fs::read_to_string(&call_log_path).expect("semantic calls should be logged");
        let relevant_lines = call_log
            .lines()
            .filter(|line| line.contains(&format!("\"sessionId\":\"{session_id}\"")))
            .collect::<Vec<_>>();
        let call_log = relevant_lines.join("\n");
        assert!(
            call_log.contains("\"attempt\":\"semantic.responses.primary\""),
            "{call_log}"
        );
        assert!(
            call_log.contains("\"attempt\":\"semantic.chat.primary\""),
            "{call_log}"
        );
        assert!(call_log.contains("\"workspacePath\""), "{call_log}");
        assert!(
            call_log.contains("\"operation\":\"semantic\""),
            "{call_log}"
        );
        assert!(call_log.contains("\"responseStatus\":404"), "{call_log}");
        assert!(call_log.contains("\"responseStatus\":200"), "{call_log}");
    }

    #[test]
    fn semantic_json_extractor_accepts_model_text_wrapped_json_object() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "星梦梦审查如下：\n```json\n{\"passed\":true,\"reviewerAgent\":\"星梦梦\",\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"通过\",\"confidence\":\"high\"}\n```\n可以继续。"
                    }
                }
            ]
        });

        let value = extract_json_value_from_model_response(&response)
            .expect("semantic JSON extractor should tolerate wrapped model text");

        assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
        assert_eq!(
            value.get("reviewerAgent").and_then(Value::as_str),
            Some("星梦梦")
        );
    }

    #[test]
    fn agent_decision_parser_accepts_minimax_think_wrapped_json() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n这里会推理 JSON schema，比如 {\"mode\":\"question|solutions\"}，但这不是最终输出。\n</think>\n{\"mode\":\"question\",\"assistantMessage\":\"我先确认电商订单系统的目标用户和核心流程。\",\"understandingSummary\":\"电商订单管理系统\",\"readiness\":{\"productType\":\"软件\",\"targetUsers\":\"电商运营人员\",\"coreProblem\":\"订单处理分散\",\"keyFeatures\":[\"订单管理\"],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"约束条件\",\"用户确认\"],\"readyForSolutions\":false}}"
                    }
                }
            ]
        });

        let decision = parse_agent_decision_response_for_provider(&response, "custom")
            .expect("Custom MiniMax think-wrapped chat responses should parse final JSON");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "我先确认电商订单系统的目标用户和核心流程。"
        );
    }

    #[test]
    fn custom_minimax_think_wrapped_plain_text_question_keeps_normal_chat_usable() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n先判断还缺目标用户。\n</think>\n我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？"
                    }
                }
            ]
        });

        let decision = parse_agent_decision_response_for_provider(&response, "custom")
            .expect("custom MiniMax plain-text follow-up after think should stay as question");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？"
        );
    }

    #[test]
    fn agent_decision_parser_does_not_apply_minimax_cleanup_to_standard_providers() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n这里会推理 JSON schema，比如 {\"mode\":\"question|solutions\"}，但这不是最终输出。\n</think>\n{\"mode\":\"question\",\"assistantMessage\":\"不应被标准渠道解析\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}"
                    }
                }
            ]
        });

        for provider in ["deepseek", "codex", "openai"] {
            let error = parse_agent_decision_response_for_provider(&response, provider)
                .expect_err("standard providers should keep rejecting this MiniMax-only wrapper");
            assert_eq!(
                error, "agent_response_not_json",
                "{provider} should not strip MiniMax reasoning tags and parse the later JSON"
            );
        }
    }

    #[test]
    fn agent_decision_parser_preserves_think_text_inside_valid_json_string() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "{\"mode\":\"question\",\"assistantMessage\":\"请保留 <think>示例</think> 文本。\",\"understandingSummary\":\"标签示例\",\"readiness\":{\"productType\":\"软件\",\"targetUsers\":\"测试用户\",\"coreProblem\":\"验证解析边界\",\"keyFeatures\":[\"解析\"],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"约束条件\",\"用户确认\"],\"readyForSolutions\":false}}"
                    }
                }
            ]
        });

        let decision = parse_agent_decision_response(&response)
            .expect("valid JSON strings should parse before reasoning cleanup is attempted");

        assert_eq!(
            decision.assistant_message,
            "请保留 <think>示例</think> 文本。"
        );
    }

    #[test]
    fn semantic_json_extractor_accepts_minimax_think_wrapped_json_object() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n星梦梦先检查 schema 形态：{\"passed\":\"boolean\"}。\n</think>\n```json\n{\"passed\":true,\"reviewerAgent\":\"星梦梦\",\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"通过\",\"confidence\":\"high\"}\n```"
                    }
                }
            ]
        });

        let value = extract_json_value_from_model_response_for_provider(&response, "custom")
            .expect("custom semantic JSON extractor should tolerate MiniMax think tags");

        assert_eq!(value.get("passed").and_then(Value::as_bool), Some(true));
        assert_eq!(
            value.get("reviewerAgent").and_then(Value::as_str),
            Some("星梦梦")
        );
    }

    #[test]
    fn semantic_json_extractor_does_not_apply_minimax_cleanup_to_standard_providers() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n星梦梦先检查 schema 形态：{\"passed\":\"boolean\"}。\n</think>\n```json\n{\"passed\":true,\"reviewerAgent\":\"星梦梦\",\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"通过\",\"confidence\":\"high\"}\n```"
                    }
                }
            ]
        });

        for provider in ["deepseek", "codex", "openai"] {
            let value = extract_json_value_from_model_response_for_provider(&response, provider)
                .expect("standard providers should use existing embedded-object extraction");
            assert_ne!(
                value.get("passed").and_then(Value::as_bool),
                Some(true),
                "{provider} should not strip MiniMax reasoning tags and parse the later JSON"
            );
        }
    }

    #[test]
    fn semantic_json_extractor_preserves_think_text_inside_valid_json_string() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "{\"passed\":true,\"reviewerAgent\":\"星梦梦\",\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"保留 <think>示例</think> 文本。\",\"confidence\":\"high\"}"
                    }
                }
            ]
        });

        let value = extract_json_value_from_model_response(&response)
            .expect("valid semantic JSON should parse before reasoning cleanup is attempted");

        assert_eq!(
            value.get("reviewSummary").and_then(Value::as_str),
            Some("保留 <think>示例</think> 文本。")
        );
    }

    #[test]
    fn custom_minimax_think_wrapped_malformed_json_is_rejected() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n这里是 MiniMax 推理过程。\n</think>\n{\"mode\":\"question\",\"assistantMessage\":"
                    }
                }
            ]
        });

        let error = parse_agent_decision_response_for_provider(&response, "custom")
            .expect_err("malformed JSON after MiniMax reasoning must not be treated as valid");

        assert_eq!(error, "agent_response_not_json");
    }

    #[test]
    fn custom_minimax_top_level_think_json_example_does_not_mask_malformed_final_json() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n示例对象不能被当成最终答案：{\"mode\":\"question\",\"assistantMessage\":\"错误来源\",\"readiness\":{\"keyFeatures\":[],\"constraints\":[],\"summaryPresented\":false,\"summaryConfirmed\":false,\"missingFields\":[\"目标用户\"],\"readyForSolutions\":false}}\n</think>\n{\"mode\":\"question\",\"assistantMessage\":"
                    }
                }
            ]
        });

        let error = parse_agent_decision_response_for_provider(&response, "custom").expect_err(
            "custom MiniMax parsing must reject malformed final JSON after top-level think",
        );

        assert_eq!(error, "agent_response_not_json");
    }

    #[test]
    fn custom_minimax_semantic_top_level_think_json_example_does_not_mask_malformed_final_json() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "<think>\n示例对象不能被当成星梦梦最终验收：{\"passed\":true,\"reviewerAgent\":\"星梦梦\",\"blockingIssues\":[],\"questionsForMengXingxing\":[],\"requiredRepairs\":[],\"reviewSummary\":\"错误来源\",\"confidence\":\"high\"}\n</think>\n{\"passed\":"
                    }
                }
            ]
        });

        let value = extract_json_value_from_model_response_for_provider(&response, "custom");

        assert!(
            value.is_none(),
            "custom semantic extraction must not parse JSON examples inside top-level MiniMax think blocks"
        );
    }

    #[test]
    fn semantic_review_recovery_accepts_truncated_self_declared_nonblocking_text() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "{\n  \"blockingIssues\": [\n    \"selectedSolution.roleRationale 中 docs 角色描述提到 Codex 后续接管入口，但 AGENTS.md 和 .codex/COORDINATOR-SUBAGENTS.md 已生成且内容正确；该表述属于计划性交接说明，不构成阻断。不构成阻断。不构成阻断。"
                    }
                }
            ]
        });

        let value = recover_semantic_review_value_from_invalid_response(&response)
            .expect("truncated self-declared nonblocking review should be recoverable");
        let review = parse_semantic_review_value(&value).unwrap();

        assert!(review.passed);
        assert!(review.blocking_issues.is_empty());
        assert!(review.review_summary.contains("截断"));
    }

    #[test]
    fn semantic_review_recovery_preserves_complete_blockers_from_truncated_json() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "{\n  \"blockingIssues\": [\n    \"selectedSolution.roleRationale 中 frontend 角色职责包含 '参与低保真线框图原型设计'，但 teamComposition 中未包含 rapid-prototyper 角色。\",\n    \"selectedSolution.roleRationale 中 backend 角色职责包含 '负责安全工程职责'，但 teamComposition 中未包含 security-engineer 角色。\",\n    \"selectedSolution.roleRationale 中 miniapp 角色职责包含 '承担UI"
                    }
                }
            ]
        });

        let value = recover_semantic_review_value_from_invalid_response(&response)
            .expect("truncated review should preserve complete blocking issue strings");
        let review = parse_semantic_review_value(&value).unwrap();

        assert!(!review.passed);
        assert_eq!(review.blocking_issues.len(), 2);
        assert!(review.blocking_issues[0].contains("rapid-prototyper"));
        assert!(review.blocking_issues[1].contains("security-engineer"));
        assert!(review.review_summary.contains("截断"));
    }

    #[test]
    fn semantic_review_recovery_preserves_complete_issues_alias_from_truncated_json() {
        let response = json!({
            "choices": [
                {
                    "message": {
                        "content": "{\n  \"issues\": [\n    \"selectedSolution.roleRationale 中 frontend 与低代码架构矛盾。\",\n    \"selectedSolution.roleRationale 中 backend 与低代码架构矛盾。\",\n    \"selectedSolution.roleRationale 中 database"
                    }
                }
            ]
        });

        let value = recover_semantic_review_value_from_invalid_response(&response)
            .expect("truncated issues alias should preserve complete issue strings");
        let review = parse_semantic_review_value(&value).unwrap();

        assert!(!review.passed);
        assert_eq!(review.blocking_issues.len(), 2);
        assert!(review.blocking_issues[0].contains("frontend"));
        assert!(review.blocking_issues[1].contains("backend"));
    }

    #[test]
    fn semantic_review_parser_treats_missing_passed_with_blockers_as_failed_review() {
        let value = json!({
            "blockingIssues": ["AGENTS.md 与 selectedSolution 明显矛盾，必须修复。"],
            "requiredRepairs": ["修复 AGENTS.md 工作流。"]
        });

        let review = parse_semantic_review_value(&value)
            .expect("semantic review parser should preserve blockers from partial JSON");

        assert!(!review.passed);
        assert_eq!(review.blocking_issues.len(), 1);
        assert_eq!(review.required_repairs.len(), 1);
        assert!(review.review_summary.contains("结构不完整"));
    }

    #[test]
    fn agent_network_failures_surface_actionable_message() {
        let message = map_agent_http_error(&AgentHttpError {
            status_code: 0,
            body: "error sending request for url (https://api.deepseek.com/v1/chat/completions): dns error".to_string(),
            endpoint: None,
        });

        assert!(message.contains("模型请求失败"));
        assert!(message.contains("网络"));
        assert!(message.contains("Base URL"));
        assert!(message.contains("dns error"));
        assert!(!message.contains("agent_request_failed"));
    }

    #[test]
    fn invalid_success_response_error_keeps_sanitized_debug_detail() {
        let message = map_agent_http_error(&AgentHttpError {
            status_code: 200,
            body: "invalid_json bodySnippet=data: {\"error\":\"bad\"} api_key=test-api-key-secret123456"
                .to_string(),
            endpoint: Some("https://api.deepseek.com/chat/completions".to_string()),
        });

        assert!(message.contains("agent_response_invalid"), "{message}");
        assert!(message.contains("status=200"), "{message}");
        assert!(
            message.contains("endpoint=https://api.deepseek.com/chat/completions"),
            "{message}"
        );
        assert!(message.contains("bodySnippet=data:"), "{message}");
        assert!(!message.contains("test-api-key-secret123456"), "{message}");
    }

    #[test]
    fn body_read_timeout_is_not_reported_as_empty_json_body() {
        let base_url = spawn_headers_then_delayed_body_server();
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(50))
            .timeout(Duration::from_millis(100))
            .build()
            .expect("test client should build");
        let endpoint = format!("{base_url}/chat/completions");

        let error = post_agent_json(
            &client,
            &endpoint,
            "sk-secret123456",
            &json!({ "model": "deepseek-v4-pro" }),
            None,
        )
        .expect_err("delayed response body should fail body reading");

        assert_eq!(error.status_code, 200);
        assert!(
            error.body.contains("response_body_read_failed"),
            "{}",
            error.body
        );
        assert!(
            !error.body.contains("invalid_json reason=empty_body"),
            "{}",
            error.body
        );
        assert!(!error.body.contains("sk-secret123456"), "{}", error.body);
    }

    #[test]
    fn chat_to_responses_retry_reports_responses_failure_when_both_fail() {
        let client = Client::builder()
            .connect_timeout(Duration::from_millis(100))
            .timeout(Duration::from_millis(500))
            .build()
            .expect("test client should build");
        let mut session = base_session();
        session.provider = "codex".to_string();
        session.model = "gpt-5.5".to_string();
        session.base_url = "http://127.0.0.1:1".to_string();
        session.api_key = "test-api-key".to_string();
        session.wire_api = "chat_completions".to_string();
        let previous_error = AgentHttpError {
            status_code: 0,
            body: "error sending request for url (http://127.0.0.1:1/chat/completions)".to_string(),
            endpoint: None,
        };

        let message = request_agent_decision_via_responses(
            &client,
            &session,
            Some("user_message"),
            Some(previous_error),
        )
        .expect_err("responses retry should fail against a closed local port");

        assert!(message.contains("/responses"), "{message}");
        assert!(!message.contains("/chat/completions"), "{message}");
    }

    #[test]
    fn codex_responses_main_session_does_not_fallback_to_chat_when_solutions_request_fails() {
        let paths = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_responses_failure_capture_server(paths.clone());
        let mut session = base_session();
        session.provider = "codex".to_string();
        session.model = "gpt-5.5".to_string();
        session.base_url = base_url;
        session.api_key = "test-api-key".to_string();
        session.wire_api = "responses".to_string();
        session.messages = vec![
            build_agent_message(
                "assistant",
                "目前理解为：产品形态是电商网站；目标用户是消费者和企业采购；核心问题是线上成交；关键功能包括商品、购物车、支付、企业询价；约束条件是 Web + 小程序。这样理解对吗？如果准确，我就继续整理三套方案。",
            ),
            build_agent_message("user", "是的，按这个理解，请给我三个方案。"),
        ];

        let message = request_agent_decision(&session, Some("user_message"))
            .expect_err("responses failure should surface without chat fallback");

        assert!(message.contains("/responses"), "{message}");
        assert!(!message.contains("/chat/completions"), "{message}");
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/responses"]
        );
    }

    #[test]
    fn codex_responses_config_does_not_override_deepseek_or_custom_sessions() {
        with_temp_codex_home(
            Some(r#"{ "OPENAI_API_KEY": "test-api-key-from-auth-json" }"#),
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
                let mut deepseek_session = base_session();
                deepseek_session.provider = "deepseek".to_string();
                deepseek_session.base_url = "https://api.deepseek.com/v1".to_string();
                deepseek_session.wire_api = "responses".to_string();

                let mut custom_session = base_session();
                custom_session.provider = "custom".to_string();
                custom_session.base_url = "https://openai-compatible.example/v1".to_string();
                custom_session.wire_api = "responses".to_string();

                assert_eq!(
                    effective_session_wire_api(&deepseek_session),
                    "chat_completions"
                );
                assert_eq!(
                    effective_session_wire_api(&custom_session),
                    "chat_completions"
                );
            },
        );
    }

    #[test]
    fn deepseek_agent_session_uses_chat_completions_in_main_flow() {
        let paths = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_endpoint_capture_server(paths.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "test-api-key".to_string();
        session.wire_api = "responses".to_string();

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("deepseek main flow should use chat completions");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/chat/completions"]
        );
    }

    #[test]
    fn deepseek_agent_session_retries_chat_when_primary_response_has_missing_content() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_missing_content_then_success_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "test-api-key".to_string();
        session.wire_api = "chat_completions".to_string();

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("DeepSeek chat flow should retry when primary response has no content");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "电商网站需要先确认目标用户和核心交易流程。"
        );
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(captured.len(), 2);
        assert!(captured[0].contains("\"response_format\""));
        assert!(!captured[1].contains("\"response_format\""));
    }

    #[test]
    fn deepseek_agent_session_retries_chat_when_primary_response_body_is_empty() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_empty_body_then_success_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("DeepSeek chat flow should retry when primary response body is empty");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "空响应后已恢复。");
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(captured.len(), 2);
        assert!(captured[0].contains("\"response_format\""));
        assert!(!captured[1].contains("\"response_format\""));
    }

    #[test]
    fn deepseek_agent_session_retries_minimal_when_primary_and_compat_bodies_are_empty() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_two_empty_bodies_then_success_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-pro".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();
        session.messages = vec![build_agent_message("user", "我要做学生管理系统")];

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("DeepSeek empty primary and compat bodies should retry minimal body");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "连续空响应后已恢复。");
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(captured.len(), 3);
        assert!(
            captured[0].contains("\"response_format\""),
            "{}",
            captured[0]
        );
        assert!(
            !captured[1].contains("\"response_format\""),
            "{}",
            captured[1]
        );
        assert!(captured[2].contains("You are 梦星星"), "{}", captured[2]);
    }

    #[test]
    fn plain_text_question_response_falls_back_without_contract_repair() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_plain_text_until_repair_success_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();

        let decision = request_agent_decision_with_contract_repair(&session, Some("user_message"))
            .expect(
                "plain-text question should stay usable without forcing solution-stage JSON repair",
            );

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "我先确认一下：这个网站主要给社团负责人用，还是也给普通成员使用？"
        );
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(
            captured.len(),
            1,
            "normal question-stage chat should not be forced through contract repair"
        );
        assert!(
            captured
                .iter()
                .all(|request| !request.contains("contract_repair")),
            "repair requests should be reserved for final structured solution failures"
        );
    }

    #[test]
    fn deepseek_agent_session_parses_sse_response_and_requests_non_streaming() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_sse_response_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("SSE-style successful responses should still be parsed");

        assert_eq!(decision.mode, "question");
        assert_eq!(decision.assistant_message, "继续确认双端范围");
        let captured = requests.lock().expect("request capture should be readable");
        assert!(captured[0].contains("\"stream\":false"), "{}", captured[0]);
    }

    #[test]
    fn parser_continues_to_later_plain_text_candidate_after_invalid_json_fragment() {
        let decision = parse_agent_decision_response(&json!({
            "choices": [
                {
                    "message": {
                        "content": [
                            "{\"mode\":\"question\",\"assistantMessage\":",
                            "完美，信息已经很充分了。让我总结确认，然后给出方案。"
                        ]
                    }
                }
            ]
        }))
        .expect("invalid text fragments should not hide a usable plain-text assistant reply");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            decision.assistant_message,
            "完美，信息已经很充分了。让我总结确认，然后给出方案。"
        );
    }

    #[test]
    fn deepseek_v4_pro_confirmed_summary_is_retried_as_solutions() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_confirmed_summary_then_solutions_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-pro".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();
        session.readiness = AgentReadiness {
            product_type: Some("MCP".to_string()),
            target_users: Some("企业员工".to_string()),
            core_problem: Some("移动端使用受限".to_string()),
            key_features: vec!["登录".to_string(), "聊天".to_string(), "知识库".to_string()],
            constraints: vec!["Android 8+".to_string()],
            summary_presented: true,
            summary_confirmed: false,
            missing_fields: vec!["用户确认".to_string()],
            ready_for_solutions: false,
        };
        session.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: "目前理解为：产品形态是 MCP；目标用户是企业员工；这样理解对吗？".to_string(),
            ..Default::default()
        });
        session.messages.push(AgentMessage {
            role: "user".to_string(),
            content: "确认".to_string(),
            ..Default::default()
        });
        session.readiness = merge_readiness(
            &session.readiness,
            None,
            &session.messages,
            None,
            "deepseek",
        );

        let decision = request_agent_decision_with_contract_repair(&session, Some("user_message"))
            .expect("DeepSeek v4-pro summary-after-confirmation should be forced into solutions");

        assert_eq!(decision.mode, "solutions");
        assert_eq!(decision.solutions.as_ref().map(Vec::len), Some(3));
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(captured.len(), 2);
        assert!(captured[0].contains("deepseek-v4-pro"));
        assert!(
            captured[1].contains("contract_repair_confirmed_solutions"),
            "{}",
            captured[1]
        );
    }

    #[test]
    fn deepseek_flash_prefaced_fenced_solutions_are_repaired_to_raw_solutions() {
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_prefaced_fenced_solutions_then_raw_solutions_server(requests.clone());
        let mut session = base_session();
        session.provider = "deepseek".to_string();
        session.model = "deepseek-v4-flash".to_string();
        session.base_url = base_url;
        session.api_key = "sk-test".to_string();
        session.wire_api = "chat_completions".to_string();
        session.readiness = AgentReadiness {
            product_type: Some("Android App".to_string()),
            target_users: Some("企业员工".to_string()),
            core_problem: Some("移动端使用受限".to_string()),
            key_features: vec!["登录".to_string(), "聊天".to_string(), "知识库".to_string()],
            constraints: vec!["Kotlin".to_string(), "Compose".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: "目前理解为：Android App，面向企业员工。这样理解对吗？".to_string(),
            ..Default::default()
        });
        session.messages.push(AgentMessage {
            role: "user".to_string(),
            content: "没有，为什么你会问这个？我们不是在聊项目吗？".to_string(),
            ..Default::default()
        });

        let decision = request_agent_decision_with_contract_repair(&session, Some("user_message"))
            .expect("prefaced fenced solutions should trigger contract repair, not plain chat");

        assert_eq!(decision.mode, "solutions");
        assert_eq!(decision.solutions.as_ref().map(Vec::len), Some(3));
        let captured = requests.lock().expect("request capture should be readable");
        assert_eq!(captured.len(), 2);
        assert!(
            captured[1].contains("contract_repair_unstructured_solutions"),
            "{}",
            captured[1]
        );
    }

    #[test]
    fn custom_openai_compatible_agent_session_uses_chat_completions_in_main_flow() {
        let paths = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_endpoint_capture_server(paths.clone());
        let mut session = base_session();
        session.provider = "custom".to_string();
        session.model = "custom-model".to_string();
        session.base_url = base_url;
        session.api_key = "test-api-key".to_string();
        session.wire_api = "responses".to_string();

        let decision = request_agent_decision(&session, Some("user_message"))
            .expect("custom OpenAI-compatible main flow should use chat completions");

        assert_eq!(decision.mode, "question");
        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/chat/completions"]
        );
    }

    #[test]
    fn official_codex_session_without_api_key_is_rejected_not_routed_to_cli() {
        with_temp_codex_home(
            Some(
                r#"{
  "OPENAI_API_KEY": null,
  "tokens": {
    "access_token": "ey-official-codex-access-token"
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
                let mut session = base_session();
                session.provider = "codex".to_string();
                session.model = "gpt-5.5".to_string();
                session.api_key = String::new();
                session.base_url = String::new();
                session.wire_api = "responses".to_string();

                assert_eq!(
                    resolve_session_api_key("codex", "")
                        .expect_err("official Codex login is unsupported"),
                    "codex_official_login_unsupported"
                );
                assert_eq!(effective_session_wire_api(&session), "responses");
                assert_eq!(
                    request_agent_decision(&session, Some("user_message"))
                        .expect_err("official Codex login must not start CLI"),
                    "codex_official_login_unsupported"
                );
            },
        );
    }

    #[test]
    fn codex_session_request_uses_responses_wire_api_from_local_config_even_if_session_is_chat() {
        let paths = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_agent_endpoint_capture_server(paths.clone());

        with_temp_codex_home(
            Some(r#"{ "OPENAI_API_KEY": "test-api-key-from-auth-json" }"#),
            Some(&format!(
                r#"
model_provider = "aicodewith"
model = "gpt-5.5"

[model_providers.aicodewith]
wire_api = "responses"
base_url = "{base_url}"
"#
            )),
            || {
                let mut session = base_session();
                session.provider = "codex".to_string();
                session.model = "gpt-5.5".to_string();
                session.base_url = base_url.clone();
                session.wire_api = "chat_completions".to_string();
                session.messages = vec![
                    build_agent_message("assistant", "这个理解准确吗？"),
                    build_agent_message("user", "是的"),
                ];

                let decision = request_agent_decision(&session, Some("user_message"))
                    .expect("codex configured as responses should use responses endpoint");

                assert_eq!(decision.mode, "question");
            },
        );

        assert_eq!(
            paths
                .lock()
                .expect("path capture should be readable")
                .as_slice(),
            ["/responses"]
        );
    }

    #[test]
    fn send_message_rolls_back_user_message_when_model_request_fails() {
        let store = AgentStore::new();
        let session_id = "session-request-failure".to_string();
        let mut session = base_session();
        session.base_url = "http://127.0.0.1:1".to_string();
        session.messages = vec![build_agent_message("assistant", "请先确认产品目标。")];
        let original_messages = session.messages.clone();
        store
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        let result = store.send_message(AgentSendRequest {
            session_id: session_id.clone(),
            message: "是的".to_string(),
        });

        assert!(result.is_err());
        let sessions = store.sessions.lock().unwrap();
        let stored = sessions
            .get(&session_id)
            .expect("session should remain available");
        assert_eq!(stored.messages, original_messages);
    }

    #[test]
    fn send_message_writes_runtime_diagnostic_when_agent_response_is_invalid() {
        let store = AgentStore::new();
        let session_id = "session-runtime-diagnostic".to_string();
        let workspace = repo_root()
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!(
                "agent-runtime-diagnostic-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
        fs::create_dir_all(&workspace).unwrap();
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_always_empty_body_server(requests.clone());
        let mut session = base_session();
        session.base_url = base_url;
        session.api_key = "sk-secret123456".to_string();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.messages = vec![build_agent_message("assistant", "请确认当前理解是否准确。")];
        store
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        let result = store.send_message(AgentSendRequest {
            session_id: session_id.clone(),
            message: "是的".to_string(),
        });

        let error = result.expect_err("empty 200 responses should fail");
        assert!(error.contains("agent_response_invalid"), "{error}");
        let diagnostic_path = workspace
            .join(".commonhe")
            .join("session")
            .join(&session_id)
            .join("runtime-diagnostics.jsonl");
        let diagnostic =
            fs::read_to_string(&diagnostic_path).expect("runtime diagnostic should be written");
        assert!(
            diagnostic.contains("\"operation\":\"message\""),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("\"provider\":\"deepseek\""),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("\"model\":\"deepseek-v4-flash\""),
            "{diagnostic}"
        );
        assert!(
            diagnostic.contains("agent_response_invalid"),
            "{diagnostic}"
        );
        assert!(diagnostic.contains("reason=empty_body"), "{diagnostic}");
        assert!(!diagnostic.contains("sk-secret123456"), "{diagnostic}");
    }

    #[test]
    fn send_message_writes_agent_call_log_for_each_http_attempt() {
        let store = AgentStore::new();
        let session_id = "session-agent-call-log".to_string();
        let workspace = repo_root()
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!(
                "agent-call-log-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
        let runtime_root = repo_root()
            .join("tmp")
            .join("desktop-main-flow")
            .join(format!(
                "commonhe-runtime-log-root-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&runtime_root).unwrap();
        let requests = Arc::new(TestMutex::new(Vec::new()));
        let base_url = spawn_chat_two_empty_bodies_then_success_server(requests.clone());
        let mut session = base_session();
        session.session_id = session_id.clone();
        session.base_url = base_url;
        session.api_key = "sk-secret123456".to_string();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.messages = vec![build_agent_message("assistant", "请确认当前理解是否准确。")];
        store
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        let result = with_commonhe_runtime_root(&runtime_root, || {
            store
                .send_message(AgentSendRequest {
                    session_id: session_id.clone(),
                    message: "是的".to_string(),
                })
                .expect("minimal retry should recover")
        });

        assert_eq!(
            result.messages.last().unwrap().content,
            "连续空响应后已恢复。"
        );
        let target_call_log_path = workspace
            .join("data")
            .join("logs")
            .join("commonhe-agent-calls.jsonl");
        assert!(
            !target_call_log_path.exists(),
            "model-call log should stay in the launcher runtime root, not target workspace data/logs: {}",
            target_call_log_path.display()
        );
        let call_log_path = runtime_root
            .join("data")
            .join("logs")
            .join("commonhe-agent-calls.jsonl");
        let call_log =
            fs::read_to_string(&call_log_path).expect("agent call log should be written");
        let lines = call_log
            .lines()
            .filter(|line| line.contains(&format!("\"sessionId\":\"{session_id}\"")))
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 3, "{call_log}");
        assert!(
            lines[0].contains("\"attempt\":\"agent.chat.primary\""),
            "{call_log}"
        );
        assert!(
            lines[1].contains("\"attempt\":\"agent.chat.compat\""),
            "{call_log}"
        );
        assert!(
            lines[2].contains("\"attempt\":\"agent.chat.minimal\""),
            "{call_log}"
        );
        assert!(call_log.contains("\"operation\":\"message\""), "{call_log}");
        assert!(call_log.contains("\"workspacePath\""), "{call_log}");
        assert!(call_log.contains("\"requestBody\""), "{call_log}");
        assert!(call_log.contains("\"responseStructure\""), "{call_log}");
        assert!(
            call_log.contains("\"parseError\":\"empty_body\""),
            "{call_log}"
        );
        assert!(call_log.contains("\"textCandidateCount\":1"), "{call_log}");
        assert!(!call_log.contains("sk-secret123456"), "{call_log}");
    }

    #[test]
    fn meng_xingxing_solutions_missing_role_rationale_are_rejected() {
        let raw = r#"
{
  "mode": "solutions",
  "assistantMessage": "三方案已完成。",
  "understandingSummary": "电商 Web + 小程序双端初始化协作包。",
  "readiness": {
    "productType": "电商网站",
    "targetUsers": "个人消费者、企业采购",
    "coreProblem": "提升线上成交并支持企业询价",
    "keyFeatures": ["商品列表", "购物车", "企业询价", "小程序"],
    "constraints": ["Web + 小程序"],
    "summaryPresented": true,
    "summaryConfirmed": true,
    "missingFields": [],
    "readyForSolutions": true
  },
  "solutions": [
    {
      "id": "A",
      "title": "快速方案",
      "architectureSummary": "Web + 小程序 + API",
      "teamComposition": ["frontend", "backend", "miniapp"],
      "tokenEstimate": "20k",
      "recommendationText": "快速启动"
    },
    {
      "id": "B",
      "title": "均衡方案",
      "architectureSummary": "Web + 小程序 + API + 数据库",
      "teamComposition": ["frontend", "backend", "miniapp", "database"],
      "tokenEstimate": "35k",
      "recommendationText": "推荐"
    },
    {
      "id": "C",
      "title": "企业方案",
      "architectureSummary": "Web + 小程序 + API + 数据库 + 权限",
      "teamComposition": ["architect", "frontend", "backend", "miniapp", "database"],
      "tokenEstimate": "50k",
      "recommendationText": "完整"
    }
  ]
}
"#;

        let result = parse_agent_decision_content(raw);

        assert_eq!(result.unwrap_err(), "agent_solution_role_rationale_missing");
    }

    #[test]
    fn parser_surfaces_repairable_solution_contract_errors() {
        let value = json!({
            "choices": [{
                "message": {
                    "content": r#"{
                      "mode": "solutions",
                      "assistantMessage": "三方案已完成。",
                      "readiness": {
                        "keyFeatures": [],
                        "constraints": [],
                        "summaryPresented": true,
                        "summaryConfirmed": true,
                        "missingFields": [],
                        "readyForSolutions": true
                      },
                      "solutions": [
                        {"id":"A","title":"A","architectureSummary":"架构A","teamComposition":["frontend"],"tokenEstimate":"20k","recommendationText":"A"},
                        {"id":"B","title":"B","architectureSummary":"架构B","teamComposition":["backend"],"tokenEstimate":"30k","recommendationText":"B"},
                        {"id":"C","title":"C","architectureSummary":"架构C","teamComposition":["architect"],"tokenEstimate":"40k","recommendationText":"C"}
                      ]
                    }"#
                }
            }]
        });

        let result = parse_agent_decision_response(&value);

        assert_eq!(result.unwrap_err(), "agent_solution_role_rationale_missing");
    }

    #[test]
    fn contract_repair_prompt_asks_meng_xingxing_to_fix_schema_without_user_repeat() {
        let session = base_session();

        let prompt =
            build_agent_context_prompt(&session, Some("contract_repair_missing_role_rationale"));

        assert!(prompt.contains("previous solutions response was rejected"));
        assert!(prompt.contains("Do not ask the user to repeat anything"));
        assert!(prompt.contains("roleRationale"));
        assert!(prompt.contains("omittedRoleRationale"));
    }

    #[test]
    fn agent_prompt_forbids_markdown_solution_bundles_and_fake_tool_tags() {
        let session = base_session();
        let prompt = build_agent_context_prompt(&session, Some("session_start"));

        assert!(prompt.contains("Do not include Markdown"));
        assert!(prompt.contains("<solution-picker-agent>"));
        assert!(prompt.contains("pseudo tool calls"));
        assert!(prompt.contains("desktop program opens the solution selector"));
        assert!(prompt.contains("solutions[]"));
    }

    #[test]
    fn unstructured_solution_repair_prompt_requires_json_retry_not_chat_selection() {
        let session = base_session();
        let prompt =
            build_agent_context_prompt(&session, Some("contract_repair_unstructured_solutions"));

        assert!(prompt.contains("described 方案A/方案B/方案C in Markdown"));
        assert!(prompt.contains("JSON only"));
        assert!(prompt.contains("Do not ask the user to choose in chat"));
        assert!(prompt.contains("Do not output <solution-picker-agent>"));
        assert!(prompt.contains("desktop program opens the solution selector"));
    }

    #[test]
    fn generic_contract_repair_prompt_uses_raw_json_only_acceptance_criteria() {
        let session = base_session();
        let prompt = build_agent_context_prompt(&session, Some("contract_repair"));

        assert!(prompt.contains("Return exactly one complete, valid JSON object and nothing else"));
        assert!(prompt.contains("Output must be raw JSON only"));
        assert!(prompt.contains("Minimum acceptance criteria"));
        assert!(prompt.contains("The response parses as JSON without errors"));
        assert!(prompt.contains("Do not omit mode or solutions[]"));
    }

    #[test]
    fn bootstrap_is_blocked_until_xing_mengmeng_final_acceptance_passes() {
        let temp = repo_tmp_dir("semantic-acceptance-required");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let orchestrator_path = temp.join("common-he-init-orchestrator.ps1");
        fs::write(
            &orchestrator_path,
            r#"
param(
    [string]$Stage,
    [string]$SessionRoot,
    [string]$Choice,
    [string]$TargetRoot,
    [switch]$Execute
)
if ($Stage -eq 'confirm') {
    [pscustomobject]@{ Stage = 'confirmed'; SessionRoot = $SessionRoot; Choice = $Choice; Message = 'confirmed' }
} elseif ($Stage -eq 'bootstrap') {
    [pscustomobject]@{
        Stage = 'implementation_ready'
        SessionRoot = $SessionRoot
        TargetRoot = $TargetRoot
        GeneratedFiles = @((Join-Path $TargetRoot 'AGENTS.md'))
        Postcheck = @{ Passed = $true }
        Message = 'should not run without semantic acceptance'
    }
} else {
    throw "Unexpected stage: $Stage"
}
"#,
        )
        .unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.orchestrator_path = orchestrator_path;
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![
            AgentSolution {
                id: "A".to_string(),
                title: "方案A".to_string(),
                architecture_summary: "架构A".to_string(),
                team_composition: vec!["frontend".to_string()],
                token_estimate: "低".to_string(),
                recommendation_text: "推荐A".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "B".to_string(),
                title: "方案B".to_string(),
                architecture_summary: "架构B".to_string(),
                team_composition: vec!["frontend".to_string(), "backend".to_string()],
                token_estimate: "中".to_string(),
                recommendation_text: "推荐B".to_string(),
                role_rationale: HashMap::new(),
                omitted_role_rationale: HashMap::new(),
            },
            AgentSolution {
                id: "C".to_string(),
                title: "方案C".to_string(),
                architecture_summary: "架构C".to_string(),
                team_composition: vec!["architect".to_string()],
                token_estimate: "高".to_string(),
                recommendation_text: "推荐C".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
        ];

        let semantic_failure = SemanticReviewResult {
            passed: false,
            blocking_issues: vec!["梦星星没有说明角色选择理由和明显候选角色不选理由。".to_string()],
            questions_for_meng_xingxing: vec!["请补齐角色取舍理由。".to_string()],
            required_repairs: vec!["补齐 roleRationale 与 omittedRoleRationale。".to_string()],
            review_summary: "星梦梦发现阻断项。".to_string(),
            confidence: "high".to_string(),
        };
        let runtime = FakeSemanticRuntime::new(vec![
            semantic_failure.clone(),
            semantic_failure.clone(),
            semantic_failure.clone(),
            semantic_failure.clone(),
            semantic_failure,
        ]);
        let result = execute_solution_bootstrap_with_runtime(&session, &runtime)
            .expect("bootstrap should return a semantic failure");

        assert_eq!(result.status, "failure");
        assert!(!result.postcheck_passed);
        assert!(result.user_facing_message.contains("星梦梦"));
        assert!(!workspace.join("AGENTS.md").exists());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn semantic_review_routes_blocking_questions_back_to_meng_xingxing_then_rechecks() {
        let temp = repo_tmp_dir("semantic-repair-loop");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&workspace).unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.selected_solution_id = Some("B".to_string());
        session.readiness = AgentReadiness {
            product_type: Some("电商 Web + 小程序".to_string()),
            target_users: Some("个人消费者、企业采购".to_string()),
            core_problem: Some("提升线上成交并支持企业批量询价采购".to_string()),
            key_features: vec![
                "商品列表/详情".to_string(),
                "购物车下单".to_string(),
                "企业询价".to_string(),
                "小程序".to_string(),
            ],
            constraints: vec!["Web 端和小程序双端".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary =
            Some("家装电器浴霸电商网站与小程序双端协作包。".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "双端电商协作方案".to_string(),
            architecture_summary: "Web 商城 + 小程序 + 后端 API + 支付/物流集成".to_string(),
            team_composition: vec![
                "frontend".to_string(),
                "backend".to_string(),
                "miniapp".to_string(),
            ],
            token_estimate: "45k".to_string(),
            recommendation_text: "推荐".to_string(),
            role_rationale: test_role_rationale(),
            omitted_role_rationale: HashMap::from([(
                "devops".to_string(),
                "当前阶段先以初始化协作包收口，部署治理进入后续阶段。".to_string(),
            )]),
        }];

        let session_root = prepare_bootstrap_session(&session).expect("session should be prepared");
        let runtime = FakeSemanticRuntime::new(vec![
            SemanticReviewResult {
                passed: false,
                blocking_issues: vec!["缺少 qa/reviewer 的明确取舍说明。".to_string()],
                questions_for_meng_xingxing: vec![
                    "Web+小程序双端电商为什么不需要 qa/reviewer，还是需要补入？".to_string(),
                ],
                required_repairs: vec!["补齐 qa/reviewer 的选择或不选理由。".to_string()],
                review_summary: "星梦梦第一轮阻断。".to_string(),
                confidence: "high".to_string(),
            },
            SemanticReviewResult {
                passed: true,
                blocking_issues: vec![],
                questions_for_meng_xingxing: vec![],
                required_repairs: vec![],
                review_summary: "星梦梦复审通过。".to_string(),
                confidence: "high".to_string(),
            },
        ]);

        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = run_semantic_acceptance_gate_with_runtime(
            &mut session,
            &session_root,
            &runtime,
            &context,
        )
        .expect("semantic repair loop should complete");

        assert!(review.passed);
        assert_eq!(session.dialogue_round_count, 2);
        let selected = session
            .solutions
            .iter()
            .find(|solution| solution.id == "B")
            .expect("selected solution should still exist");
        assert!(selected.role_rationale.contains_key("qa"));
        assert!(selected.role_rationale.contains_key("reviewer"));
        let dialogue =
            fs::read_to_string(session_root.join("agent-dialogue-rounds.jsonl")).unwrap();
        assert_eq!(dialogue.lines().count(), 2);
        let final_acceptance: Value = serde_json::from_str(
            &fs::read_to_string(session_root.join("final-acceptance.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            final_acceptance.get("passed").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            final_acceptance.get("reviewRounds").and_then(Value::as_u64),
            Some(2)
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn completed_snapshot_marks_solution_selector_tool_as_completed() {
        let mut session = base_session();
        session.solutions = vec![
            AgentSolution {
                id: "A".to_string(),
                title: "方案A".to_string(),
                architecture_summary: "架构A".to_string(),
                team_composition: vec!["pm".to_string()],
                token_estimate: "低".to_string(),
                recommendation_text: "推荐A".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "B".to_string(),
                title: "方案B".to_string(),
                architecture_summary: "架构B".to_string(),
                team_composition: vec!["pm".to_string()],
                token_estimate: "中".to_string(),
                recommendation_text: "推荐B".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "C".to_string(),
                title: "方案C".to_string(),
                architecture_summary: "架构C".to_string(),
                team_composition: vec!["pm".to_string()],
                token_estimate: "高".to_string(),
                recommendation_text: "推荐C".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
        ];
        session.selected_solution_id = Some("B".to_string());
        session.finished = true;

        let snapshot = snapshot_from_session("session-2", &session);

        assert_eq!(snapshot.stage, "completed");
        assert_eq!(snapshot.tool_calls.len(), 1);
        assert_eq!(snapshot.tool_calls[0].tool_name, "open_solution_selector");
        assert_eq!(snapshot.tool_calls[0].status, "completed");
    }

    #[test]
    fn execute_solution_bootstrap_generates_workspace_files_on_success() {
        let temp = repo_tmp_dir("agent-bootstrap-success");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let orchestrator_path = temp.join("common-he-init-orchestrator.ps1");
        fs::write(
            &orchestrator_path,
            r#"
param(
    [string]$Stage,
    [string]$SessionRoot,
    [string]$Choice,
    [string]$TargetRoot,
    [switch]$Execute
)
if ($Stage -eq 'confirm') {
    [pscustomobject]@{
        Stage = 'confirmed'
        SessionRoot = $SessionRoot
        Choice = $Choice
        Message = 'confirmed'
    }
} elseif ($Stage -eq 'bootstrap') {
    New-Item -ItemType Directory -Path (Join-Path $TargetRoot 'docs') -Force | Out-Null
    Set-Content -Path (Join-Path $TargetRoot 'AGENTS.md') -Value 'ok'
    Set-Content -Path (Join-Path $TargetRoot 'docs\project_context.md') -Value 'ok'
    [pscustomobject]@{
        Stage = 'implementation_ready'
        SessionRoot = $SessionRoot
        TargetRoot = $TargetRoot
        GeneratedFiles = @(
            (Join-Path $TargetRoot 'AGENTS.md'),
            (Join-Path $TargetRoot 'docs\project_context.md')
        )
        Postcheck = @{ Passed = $true }
        HandoffPath = (Join-Path $TargetRoot '.commonhe\session\bootstrap-handoff.md')
        Message = '初始化成功，工作区内容已生成。'
    }
} else {
    throw "Unexpected stage: $Stage"
}
"#,
        )
        .unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.orchestrator_path = orchestrator_path;
        session.selected_solution_id = Some("B".to_string());
        session.readiness = AgentReadiness {
            product_type: Some("网站".to_string()),
            target_users: Some("商家".to_string()),
            core_problem: Some("快速搭建店铺".to_string()),
            key_features: vec!["商品展示".to_string(), "下单".to_string()],
            constraints: vec!["Web".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary = Some("为商家快速搭建店铺网站。".to_string());
        session.solutions = vec![
            AgentSolution {
                id: "A".to_string(),
                title: "方案A".to_string(),
                architecture_summary: "架构A".to_string(),
                team_composition: vec!["前端开发者".to_string()],
                token_estimate: "低".to_string(),
                recommendation_text: "推荐A".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "B".to_string(),
                title: "方案B".to_string(),
                architecture_summary: "架构B".to_string(),
                team_composition: vec!["前端开发者".to_string(), "后端开发者".to_string()],
                token_estimate: "中".to_string(),
                recommendation_text: "推荐B".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "C".to_string(),
                title: "方案C".to_string(),
                architecture_summary: "架构C".to_string(),
                team_composition: vec!["架构师".to_string()],
                token_estimate: "高".to_string(),
                recommendation_text: "推荐C".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
        ];

        let runtime = FakeSemanticRuntime::passing();
        let result = execute_solution_bootstrap_with_runtime(&session, &runtime)
            .expect("bootstrap should succeed");

        assert_eq!(result.status, "success");
        assert!(result.postcheck_passed);
        assert!(workspace.join("AGENTS.md").is_file());
        assert!(workspace.join("docs").join("project_context.md").is_file());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn execute_solution_bootstrap_blocks_when_final_semantic_review_rejects_generated_package() {
        let temp = repo_tmp_dir("agent-bootstrap-final-semantic-failure");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let orchestrator_path = temp.join("common-he-init-orchestrator.ps1");
        fs::write(
            &orchestrator_path,
            r#"
param(
    [string]$Stage,
    [string]$SessionRoot,
    [string]$Choice,
    [string]$TargetRoot,
    [switch]$Execute
)
if ($Stage -eq 'confirm') {
    [pscustomobject]@{
        Stage = 'confirmed'
        SessionRoot = $SessionRoot
        Choice = $Choice
        Message = 'confirmed'
    }
} elseif ($Stage -eq 'bootstrap') {
    New-Item -ItemType Directory -Path (Join-Path $TargetRoot 'docs') -Force | Out-Null
    Set-Content -Path (Join-Path $TargetRoot 'AGENTS.md') -Value 'ok'
    Set-Content -Path (Join-Path $TargetRoot 'docs\project_context.md') -Value 'ok'
    [pscustomobject]@{
        Stage = 'implementation_ready'
        SessionRoot = $SessionRoot
        TargetRoot = $TargetRoot
        GeneratedFiles = @(
            (Join-Path $TargetRoot 'AGENTS.md'),
            (Join-Path $TargetRoot 'docs\project_context.md')
        )
        Postcheck = @{ Passed = $true }
        HandoffPath = (Join-Path $TargetRoot '.commonhe\session\bootstrap-handoff.md')
        Message = '初始化成功，工作区内容已生成。'
    }
} else {
    throw "Unexpected stage: $Stage"
}
"#,
        )
        .unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.orchestrator_path = orchestrator_path;
        session.selected_solution_id = Some("B".to_string());
        session.readiness = AgentReadiness {
            product_type: Some("网站".to_string()),
            target_users: Some("商家".to_string()),
            core_problem: Some("快速搭建店铺".to_string()),
            key_features: vec!["商品展示".to_string(), "下单".to_string()],
            constraints: vec!["Web".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary = Some("为商家快速搭建店铺网站。".to_string());
        session.solutions = vec![
            AgentSolution {
                id: "A".to_string(),
                title: "方案A".to_string(),
                architecture_summary: "架构A".to_string(),
                team_composition: vec!["前端开发者".to_string()],
                token_estimate: "低".to_string(),
                recommendation_text: "推荐A".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "B".to_string(),
                title: "方案B".to_string(),
                architecture_summary: "架构B".to_string(),
                team_composition: vec!["前端开发者".to_string(), "后端开发者".to_string()],
                token_estimate: "中".to_string(),
                recommendation_text: "推荐B".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "C".to_string(),
                title: "方案C".to_string(),
                architecture_summary: "架构C".to_string(),
                team_composition: vec!["架构师".to_string()],
                token_estimate: "高".to_string(),
                recommendation_text: "推荐C".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
        ];

        let runtime = FakeSemanticRuntime::new(vec![
            SemanticReviewResult {
                passed: true,
                blocking_issues: vec![],
                questions_for_meng_xingxing: vec![],
                required_repairs: vec![],
                review_summary: "星梦梦方案预审通过。".to_string(),
                confidence: "high".to_string(),
            },
            final_package_failure_review(),
            final_package_failure_review(),
            final_package_failure_review(),
            final_package_failure_review(),
            final_package_failure_review(),
        ]);

        let result = execute_solution_bootstrap_with_runtime(&session, &runtime)
            .expect("bootstrap should return structured semantic failure");

        assert_eq!(result.status, "failure");
        assert!(result
            .user_facing_message
            .contains("最终协作包没有保留用户原始需求"));
        let final_acceptance: Value = serde_json::from_str(
            &fs::read_to_string(workspace.join(".commonhe/session/final-acceptance.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            final_acceptance.get("passed").and_then(Value::as_bool),
            Some(false)
        );
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn execute_solution_bootstrap_reports_failure_without_fake_success() {
        let temp = repo_tmp_dir("agent-bootstrap-failure");
        let workspace = temp.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        let orchestrator_path = temp.join("common-he-init-orchestrator.ps1");
        fs::write(
            &orchestrator_path,
            r#"
param(
    [string]$Stage,
    [string]$SessionRoot,
    [string]$Choice,
    [string]$TargetRoot,
    [switch]$Execute
)
if ($Stage -eq 'confirm') {
    [pscustomobject]@{
        Stage = 'confirmed'
        SessionRoot = $SessionRoot
        Choice = $Choice
        Message = 'confirmed'
    }
} elseif ($Stage -eq 'bootstrap') {
    [pscustomobject]@{
        Stage = 'postcheck_failed'
        SessionRoot = $SessionRoot
        TargetRoot = $TargetRoot
        GeneratedFiles = @()
        Postcheck = @{ Passed = $false }
        HandoffPath = (Join-Path $TargetRoot '.commonhe\session\bootstrap-handoff.md')
        Message = '初始化生成已完成，但 postcheck 未通过。'
    }
} else {
    throw "Unexpected stage: $Stage"
}
"#,
        )
        .unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.orchestrator_path = orchestrator_path;
        session.selected_solution_id = Some("A".to_string());
        session.readiness = AgentReadiness {
            product_type: Some("网站".to_string()),
            target_users: Some("商家".to_string()),
            core_problem: Some("快速搭建店铺".to_string()),
            key_features: vec!["商品展示".to_string(), "下单".to_string()],
            constraints: vec!["Web".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary = Some("为商家快速搭建店铺网站。".to_string());
        session.solutions = vec![
            AgentSolution {
                id: "A".to_string(),
                title: "方案A".to_string(),
                architecture_summary: "架构A".to_string(),
                team_composition: vec!["前端开发者".to_string()],
                token_estimate: "低".to_string(),
                recommendation_text: "推荐A".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "B".to_string(),
                title: "方案B".to_string(),
                architecture_summary: "架构B".to_string(),
                team_composition: vec!["后端开发者".to_string()],
                token_estimate: "中".to_string(),
                recommendation_text: "推荐B".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
            AgentSolution {
                id: "C".to_string(),
                title: "方案C".to_string(),
                architecture_summary: "架构C".to_string(),
                team_composition: vec!["架构师".to_string()],
                token_estimate: "高".to_string(),
                recommendation_text: "推荐C".to_string(),
                role_rationale: test_role_rationale(),
                omitted_role_rationale: test_omitted_role_rationale(),
            },
        ];

        let runtime = FakeSemanticRuntime::passing();
        let result = execute_solution_bootstrap_with_runtime(&session, &runtime)
            .expect("bootstrap should return structured failure");

        assert_eq!(result.status, "failure");
        assert!(!result.postcheck_passed);
        assert!(result.generated_files.is_empty());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn bootstrap_session_uses_confirmed_project_name_and_target_client() {
        let temp = repo_tmp_dir("agent-bootstrap-target-client");
        let workspace = temp.join("workspace-folder-name");
        fs::create_dir_all(&workspace).unwrap();

        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.selected_solution_id = Some("B".to_string());
        session.project_name = Some("学生管理协作包".to_string());
        session.target_client = Some(TargetClient::Codex);
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "双端学生管理协作方案".to_string(),
            architecture_summary: "Web 管理后台 + 小程序 + 后端 API".to_string(),
            team_composition: vec!["frontend".to_string(), "backend".to_string()],
            token_estimate: "32k".to_string(),
            recommendation_text: "推荐方案 B".to_string(),
            role_rationale: test_role_rationale(),
            omitted_role_rationale: test_omitted_role_rationale(),
        }];
        session.selected_capabilities = vec![
            SelectedCapability {
                id: "superpowers".to_string(),
                label: "superpowers".to_string(),
                recommended: true,
                selected: true,
                status: "fallback".to_string(),
                detail: "bundled".to_string(),
            },
            SelectedCapability {
                id: "chrome-devtools".to_string(),
                label: "chrome-devtools".to_string(),
                recommended: true,
                selected: false,
                status: "skipped".to_string(),
                detail: "user skipped".to_string(),
            },
        ];
        session.readiness = AgentReadiness {
            product_type: Some("软件".to_string()),
            target_users: Some("老师".to_string()),
            core_problem: Some("管理学生信息".to_string()),
            key_features: vec!["学生档案".to_string(), "成绩管理".to_string()],
            constraints: vec!["Web 和小程序".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary = Some("面向老师的学生管理系统初始化协作包。".to_string());

        let session_root = prepare_bootstrap_session(&session).expect("session should be prepared");
        let answers: Value =
            serde_json::from_str(&fs::read_to_string(session_root.join("answers.json")).unwrap())
                .unwrap();
        let decision: Value =
            serde_json::from_str(&fs::read_to_string(session_root.join("decision.json")).unwrap())
                .unwrap();
        let status: Value =
            serde_json::from_str(&fs::read_to_string(session_root.join("status.json")).unwrap())
                .unwrap();

        assert_eq!(
            answers.get("project_name").and_then(Value::as_str),
            Some("学生管理协作包")
        );
        assert_eq!(
            decision.get("project_name").and_then(Value::as_str),
            Some("学生管理协作包")
        );
        assert_eq!(
            decision.get("target_client").and_then(Value::as_str),
            Some("codex")
        );
        assert_eq!(
            decision.get("selected_solution_id").and_then(Value::as_str),
            Some("B")
        );
        assert_eq!(
            decision
                .get("selected_solution_title")
                .and_then(Value::as_str),
            Some("双端学生管理协作方案")
        );
        assert_eq!(
            decision
                .get("solution_architecture_summary")
                .and_then(Value::as_str),
            Some("Web 管理后台 + 小程序 + 后端 API")
        );
        assert!(
            decision
                .get("role_rationale")
                .and_then(Value::as_object)
                .is_some_and(|rationale| rationale.contains_key("frontend")),
            "decision must persist selected solution role rationale"
        );
        assert!(
            decision
                .get("omitted_role_rationale")
                .and_then(Value::as_object)
                .is_some_and(|rationale| rationale.contains_key("qa")),
            "decision must persist selected solution omitted role rationale"
        );
        assert_eq!(
            status.get("target_client").and_then(Value::as_str),
            Some("codex")
        );
        assert_ne!(
            decision.get("project_name").and_then(Value::as_str),
            workspace.file_name().and_then(|value| value.to_str())
        );
        let selected_capabilities = decision
            .get("selected_capabilities")
            .and_then(Value::as_array)
            .expect("selected capabilities must be serialized");
        assert_eq!(selected_capabilities.len(), 5);
        for capability_id in [
            "superpowers",
            "agent-browser",
            "chrome-devtools",
            "GitNexus",
            "Speckit",
        ] {
            let capability = selected_capabilities
                .iter()
                .find(|capability| {
                    capability.get("id").and_then(Value::as_str) == Some(capability_id)
                })
                .unwrap_or_else(|| panic!("missing mandatory capability {capability_id}"));
            assert_eq!(
                capability.get("selected").and_then(Value::as_bool),
                Some(true)
            );
            assert_ne!(
                capability.get("status").and_then(Value::as_str),
                Some("skipped")
            );
        }

        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn proposal_infers_web_miniapp_delivery_and_miniapp_role() {
        let mut session = base_session();
        session.readiness = AgentReadiness {
            product_type: Some("学生管理系统（Web + 小程序）".to_string()),
            target_users: Some("老师".to_string()),
            core_problem: Some("统一管理学生档案、考勤、成绩、作业和缴费".to_string()),
            key_features: vec![
                "学生档案".to_string(),
                "成绩管理".to_string(),
                "小程序端查询".to_string(),
            ],
            constraints: vec!["支持 Web 端".to_string(), "支持微信小程序端".to_string()],
            summary_presented: true,
            summary_confirmed: true,
            missing_fields: vec![],
            ready_for_solutions: true,
        };
        session.understanding_summary =
            Some("面向老师的学生管理系统，需同时支持 Web 和小程序。".to_string());
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "Web + 小程序协同方案".to_string(),
            architecture_summary: "Web 管理后台 + 微信小程序端 + 后端 API".to_string(),
            team_composition: vec![
                "架构师".to_string(),
                "前端开发者".to_string(),
                "后端开发者".to_string(),
                "微信小程序开发者".to_string(),
                "测试".to_string(),
            ],
            token_estimate: "约 30k".to_string(),
            recommendation_text: "推荐".to_string(),
            role_rationale: test_role_rationale(),
            omitted_role_rationale: test_omitted_role_rationale(),
        };

        let option = build_proposal_option(&session, &solution, 1);

        assert_eq!(
            option.get("delivery_mode").and_then(Value::as_str),
            Some("web-miniapp")
        );
        assert_eq!(
            option.get("architecture_summary").and_then(Value::as_str),
            Some("Web 管理后台 + 微信小程序端 + 后端 API")
        );
        assert_eq!(
            option.get("token_estimate").and_then(Value::as_str),
            Some("约 30k")
        );
        let roles = option
            .get("enabled_roles")
            .and_then(Value::as_array)
            .expect("roles should be an array");
        assert!(roles.iter().any(|role| role.as_str() == Some("miniapp")));
    }

    #[test]
    fn start_solution_bootstrap_requires_project_name_and_supported_client() {
        let store = AgentStore::new();
        let session_id = "session-target-client".to_string();
        let mut session = base_session();
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "方案B".to_string(),
            architecture_summary: "架构B".to_string(),
            team_composition: vec!["pm".to_string()],
            token_estimate: "中".to_string(),
            recommendation_text: "推荐B".to_string(),
            role_rationale: test_role_rationale(),
            omitted_role_rationale: test_omitted_role_rationale(),
        }];
        store
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        let missing_name = store.start_solution_bootstrap(AgentChooseRequest {
            session_id: session_id.clone(),
            solution_id: "B".to_string(),
            project_name: " ".to_string(),
            target_client: "codex".to_string(),
            selected_capabilities: vec![],
        });
        assert_eq!(missing_name.unwrap_err(), "agent_project_name_required");

        let unsupported_client = store.start_solution_bootstrap(AgentChooseRequest {
            session_id,
            solution_id: "B".to_string(),
            project_name: "学生管理协作包".to_string(),
            target_client: "cursor".to_string(),
            selected_capabilities: vec![],
        });
        assert_eq!(
            unsupported_client.unwrap_err(),
            "agent_target_client_unsupported"
        );
    }

    #[test]
    fn selected_solution_is_aligned_to_generated_package_roles() {
        let store = AgentStore::new();
        let session_id = "session-role-alignment".to_string();
        let mut session = base_session();
        session.solutions = vec![AgentSolution {
            id: "A".to_string(),
            title: "低代码快速搭建方案".to_string(),
            architecture_summary: "Next.js + Notion API + Vercel".to_string(),
            team_composition: vec![
                "前端开发者".to_string(),
                "DevOps自动化师".to_string(),
                "产品经理".to_string(),
            ],
            token_estimate: "约 14K tokens".to_string(),
            recommendation_text: "适合一周 MVP。".to_string(),
            role_rationale: HashMap::from([
                (
                    "前端开发者".to_string(),
                    "负责页面和 Notion API 集成。".to_string(),
                ),
                (
                    "DevOps自动化师".to_string(),
                    "负责 Vercel 部署。".to_string(),
                ),
                ("产品经理".to_string(), "负责范围控制。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([(
                "后端架构师".to_string(),
                "方案使用 Notion，无需自建服务端。".to_string(),
            )]),
        }];
        store
            .sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session);

        let snapshot = store
            .start_solution_bootstrap(AgentChooseRequest {
                session_id,
                solution_id: "A".to_string(),
                project_name: "校园社团协作包".to_string(),
                target_client: "codex".to_string(),
                selected_capabilities: vec![],
            })
            .expect("solution bootstrap should normalize selected solution roles");

        let selected = snapshot
            .solutions
            .iter()
            .find(|solution| solution.id == "A")
            .unwrap();
        for expected_role in ["backend", "devops", "docs", "frontend", "reviewer"] {
            assert!(selected
                .team_composition
                .iter()
                .any(|role| role == expected_role));
            assert!(selected.role_rationale.contains_key(expected_role));
        }
        assert!(!selected.omitted_role_rationale.contains_key("后端架构师"));
        assert!(selected.omitted_role_rationale.contains_key("产品经理"));
    }

    #[test]
    fn generated_package_role_alignment_clarifies_product_manager_omission() {
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "标准电商方案".to_string(),
            architecture_summary: "Web + 小程序 + API".to_string(),
            team_composition: vec![
                "前端开发者".to_string(),
                "后端开发者".to_string(),
                "产品经理".to_string(),
            ],
            token_estimate: "约 20K tokens".to_string(),
            recommendation_text: "适合首轮实施规划。".to_string(),
            role_rationale: HashMap::new(),
            omitted_role_rationale: HashMap::new(),
        };

        let aligned = align_solution_with_package_roles(&solution, "balanced", Some("web-miniapp"));
        let product_manager_reason = aligned
            .omitted_role_rationale
            .get("产品经理")
            .expect("product manager should be explicitly omitted when not a package role");

        assert!(product_manager_reason.contains("architect"));
        assert!(product_manager_reason.contains("产品决策"));
        assert!(!product_manager_reason.contains("已由梦星星与真源文档承接"));
        assert!(aligned
            .role_rationale
            .get("docs")
            .expect("docs role should be explained")
            .contains("接管入口文件"));
    }

    #[test]
    fn proposal_option_uses_generated_package_roles_for_team_composition() {
        let mut session = base_session();
        session.readiness.constraints = vec!["小程序、web端双端".to_string()];
        let solution = AgentSolution {
            id: "A".to_string(),
            title: "低代码快速搭建方案".to_string(),
            architecture_summary: "Next.js + Notion API + Vercel".to_string(),
            team_composition: vec![
                "前端开发者".to_string(),
                "DevOps自动化师".to_string(),
                "产品经理".to_string(),
            ],
            token_estimate: "约 14K tokens".to_string(),
            recommendation_text: "适合一周 MVP。".to_string(),
            role_rationale: HashMap::new(),
            omitted_role_rationale: HashMap::new(),
        };

        let option = build_proposal_option(&session, &solution, 0);
        let team = option
            .get("team_composition")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert!(team.contains(&"backend"));
        assert!(team.contains(&"devops"));
        assert!(team.contains(&"docs"));
        assert!(team.contains(&"frontend"));
        assert!(team.contains(&"miniapp"));
        assert!(team.contains(&"qa"));
        assert!(team.contains(&"reviewer"));
        assert!(option
            .get("agent_authored_team_composition")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .any(|role| role.as_str() == Some("产品经理")));
    }

    #[test]
    fn package_roles_include_compliance_when_role_rationale_requires_it() {
        let session = base_session();
        let mut role_rationale = HashMap::new();
        role_rationale.insert(
            "compliance".to_string(),
            "负责 OAuth2.0、API 网关、数据加密、审计日志和等保合规。".to_string(),
        );
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统企业级方案".to_string(),
            architecture_summary: "Django/Spring Boot + PostgreSQL + Web 管理后台".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "frontend".to_string(),
                "qa".to_string(),
            ],
            token_estimate: "约 70k".to_string(),
            recommendation_text: "适合长期维护。".to_string(),
            role_rationale,
            omitted_role_rationale: HashMap::new(),
        };

        let option = build_proposal_option(&session, &solution, 1);
        let team = option
            .get("team_composition")
            .and_then(Value::as_array)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert!(team.contains(&"compliance"));
    }

    #[test]
    fn final_review_refreshes_capabilities_from_generated_decision() {
        let workspace = repo_tmp_dir("final-review-capability-state");
        let session_root = workspace.join(".commonhe").join("session");
        fs::create_dir_all(&session_root).unwrap();
        fs::write(
            session_root.join("decision.json"),
            serde_json::to_string_pretty(&json!({
                "selected_capabilities": [
                    {
                        "id": "agent-browser",
                        "label": "agent-browser",
                        "recommended": true,
                        "selected": true,
                        "status": "available",
                        "detail": "found: test browser config"
                    },
                    {
                        "id": "chrome-devtools",
                        "label": "chrome-devtools",
                        "recommended": true,
                        "selected": true,
                        "status": "available",
                        "detail": "found: test mcp config"
                    },
                    {
                        "id": "GitNexus",
                        "label": "GitNexus",
                        "recommended": true,
                        "selected": true,
                        "status": "available",
                        "detail": "command ok"
                    },
                    {
                        "id": "Speckit",
                        "label": "Speckit",
                        "recommended": true,
                        "selected": true,
                        "status": "available",
                        "detail": "command ok"
                    },
                    {
                        "id": "superpowers",
                        "label": "superpowers",
                        "recommended": true,
                        "selected": true,
                        "status": "available",
                        "detail": "skills found"
                    }
                ]
            }))
            .unwrap(),
        )
        .unwrap();
        let mut session = base_session();
        session.workspace_path = workspace.to_string_lossy().to_string();
        session.selected_capabilities = resolve_selected_capabilities(vec![]);

        refresh_session_capabilities_from_generated_decision(&mut session);

        assert!(session
            .selected_capabilities
            .iter()
            .all(|capability| capability.status == "available"));
        assert!(session
            .selected_capabilities
            .iter()
            .any(|capability| capability.detail == "found: test browser config"));
    }

    #[test]
    fn repair_parser_accepts_partial_updated_solution_patch_from_provider() {
        let mut selected_solution = AgentSolution {
            id: "B".to_string(),
            title: "标准MVP".to_string(),
            architecture_summary: "Web 前端 + API 后端 + 数据库".to_string(),
            team_composition: vec!["前端开发者".to_string(), "后端开发者".to_string()],
            token_estimate: "约 18k tokens".to_string(),
            recommendation_text: "适合作为正式产品起点。".to_string(),
            role_rationale: HashMap::new(),
            omitted_role_rationale: HashMap::new(),
        };
        selected_solution
            .omitted_role_rationale
            .insert("qa".to_string(), "MVP 先由开发自测。".to_string());
        let value = json!({
            "round": 1,
            "status": "repaired",
            "issues": ["梦星星没有说明角色选择理由和明显候选角色不选理由。"],
            "response_summary": "已补齐角色取舍说明。",
            "updated_solution": {
                "role_rationale": {
                    "前端开发者": "负责 Web 端活动报名、资料库和通知界面。",
                    "后端开发者": "负责账号、活动、报名和资料数据持久化。"
                },
                "omitted_role_rationale": {
                    "security": "首轮不处理高敏合规，先保留基础鉴权和权限边界。",
                    "devops": "首轮可用托管平台部署，暂不引入专职运维角色。"
                },
                "team_composition": ["前端开发者", "后端开发者", "测试/验收"]
            }
        });

        let repair = parse_repair_decision_value(&value, 1, Some(&selected_solution)).unwrap();

        assert_eq!(repair.response_summary, "已补齐角色取舍说明。");
        let updated = repair
            .updated_solution
            .expect("partial patch should be merged into selected solution");
        assert_eq!(updated.id, "B");
        assert_eq!(updated.title, "标准MVP");
        assert!(updated
            .team_composition
            .iter()
            .any(|role| role == "测试/验收"));
        assert!(updated.role_rationale.contains_key("前端开发者"));
        assert!(updated.omitted_role_rationale.contains_key("security"));
    }

    #[test]
    fn repaired_solution_cannot_omit_selected_team_roles() {
        let mut session = base_session();
        session.selected_solution_id = Some("A".to_string());
        let updated_solution = AgentSolution {
            id: "A".to_string(),
            title: "方案A".to_string(),
            architecture_summary: "协作包规划方案".to_string(),
            team_composition: vec!["前端开发者".to_string(), "UX架构师".to_string()],
            token_estimate: "约 8k tokens".to_string(),
            recommendation_text: "用于后续实施规划。".to_string(),
            role_rationale: HashMap::from([
                ("前端开发者".to_string(), "负责界面实现。".to_string()),
                ("UX架构师".to_string(), "负责信息架构。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([
                ("UX架构师".to_string(), "保留，不省略。".to_string()),
                (
                    "安全工程师".to_string(),
                    "MVP 暂不设置专职安全角色。".to_string(),
                ),
            ]),
        };
        session.solutions = vec![updated_solution.clone()];

        apply_repaired_solution(&mut session, updated_solution).unwrap();

        let selected = session.solutions.first().unwrap();
        assert!(!selected.omitted_role_rationale.contains_key("UX架构师"));
        assert!(selected.omitted_role_rationale.contains_key("安全工程师"));
    }

    #[test]
    fn repaired_solution_normalizes_docs_marketing_and_token_budget_for_live_review() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "Uni-app跨端 + 后端Python + 快速MVP".to_string(),
            architecture_summary: "产品主名称：星星的vibecoding启动器。Uni-app + FastAPI".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: "初始化协作包规划、评审、交接约10万token。".to_string(),
            recommendation_text: "平衡方案。".to_string(),
            role_rationale: HashMap::from([
                (
                    "docs".to_string(),
                    "维护初始化协作包文档，包括AGENTS.md和.codex文件（位于项目根目录），作为Codex后续接管的入口。".to_string(),
                ),
                ("frontend".to_string(), "负责Web端和小程序的前端开发，基于开源商城主题定制。".to_string()),
                ("backend".to_string(), "负责 API 与 AI 导购问答能力集成。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([
                (
                    "marketing-content-creator".to_string(),
                    "内容创建职责由docs角色在文档中覆盖，初期无需独立内容角色。".to_string(),
                ),
                (
                    "engineering-ai-engineer".to_string(),
                    "初期导购问答用人工客服，AI后续迭代。".to_string(),
                ),
            ]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(!solution
            .architecture_summary
            .contains("产品主名称：星星的vibecoding启动器"));
        let docs_rationale = solution
            .role_rationale
            .get("docs")
            .expect("docs rationale should be preserved");
        assert!(docs_rationale.contains("目标软件"));
        assert!(docs_rationale.contains("接管入口文件"));
        assert!(!docs_rationale.contains(".codex/COORDINATOR-SUBAGENTS.md"));
        assert_eq!(
            solution
                .omitted_role_rationale
                .get("marketing-content-creator")
                .map(String::as_str),
            Some("该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色。")
        );
        assert!(solution.token_estimate.contains("不包含业务代码生成"));
        assert!(solution.token_estimate.contains("星梦梦语义验收"));
        let frontend_rationale = solution
            .role_rationale
            .get("frontend")
            .expect("frontend rationale should be preserved");
        assert!(frontend_rationale.contains("Web"));
        assert!(!frontend_rationale.contains("小程序"));
    }

    #[test]
    fn semantic_review_sanitizer_drops_unsupported_omitted_role_blocker() {
        let mut session = base_session();
        session.selected_solution_id = Some("A".to_string());
        session.solutions = vec![AgentSolution {
            id: "A".to_string(),
            title: "方案A".to_string(),
            architecture_summary: "Next.js + API handoff plan".to_string(),
            team_composition: vec![
                "engineering-frontend-developer".to_string(),
                "marketing-content-creator".to_string(),
            ],
            token_estimate: "规划与审查约 15K tokens".to_string(),
            recommendation_text: "初始化协作包规划。".to_string(),
            role_rationale: HashMap::from([
                (
                    "engineering-frontend-developer".to_string(),
                    "负责 Web 页面。".to_string(),
                ),
                (
                    "marketing-content-creator".to_string(),
                    "负责帮助文档。".to_string(),
                ),
            ]),
            omitted_role_rationale: HashMap::from([(
                "engineering-qa".to_string(),
                "MVP 先由团队自测。".to_string(),
            )]),
        }];
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.omittedRoleRationale 中包含了 'marketing-content-creator' 的条目，但该角色已被选入 teamComposition。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否笔误？".to_string()],
            required_repairs: vec!["移除 marketing-content-creator。".to_string()],
            review_summary: "误报。".to_string(),
            confidence: "high".to_string(),
        };

        let context = SemanticReviewContext::pre_bootstrap(&session);
        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_token_budget_wording_confirmation() {
        let session = base_session();
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution 的 tokenEstimate 声称 '规划与设计约5K tokens，角色交接文档约3K tokens'，但 reviewPhase 为 pre_bootstrap_solution_review，此时尚未生成任何文件。当前表述虽声明不包含业务代码，但 '角色交接文档' 可能被误解为已生成，需明确为 '计划中的 token 预算'。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否为计划预算？".to_string()],
            required_repairs: vec!["明确计划预算。".to_string()],
            review_summary: "措辞确认。".to_string(),
            confidence: "high".to_string(),
        };
        let context = SemanticReviewContext::pre_bootstrap(&session);

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_launcher_product_name_insertion_request() {
        let session = base_session();
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "产品主名称规则要求产品主名称必须是'星星的vibecoding启动器'，但当前selectedSolution中未提及该名称。请确认启动器对外名称是否已正确设置。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否补入启动器名称？".to_string()],
            required_repairs: vec!["在selectedSolution的architectureSummary中补充产品主名称。".to_string()],
            review_summary: "误把启动器名称要求用于业务项目。".to_string(),
            confidence: "high".to_string(),
        };
        let context = SemanticReviewContext::pre_bootstrap(&session);

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_final_session_artifact_false_negative_after_postcheck() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec![
                "AGENTS.md".to_string(),
                ".codex/COORDINATOR-SUBAGENTS.md".to_string(),
            ],
            generated_file_evidence: vec![],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "generatedFiles 中缺少 .commonhe/session/ 目录下的任何文件，但真源规则要求 session 审计产物存在。".to_string(),
                "generatedFiles 中缺少 decision.json、meng-xingxing-output.json、final-acceptance.json 等决策记录文件。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["session 文件是否生成？".to_string()],
            required_repairs: vec!["生成 session 审计产物。".to_string()],
            review_summary: "postcheck 已证明 session 产物存在。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_ai_engineer_backend_rationale_false_negative() {
        let mut session = base_session();
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "AI 导购电商方案".to_string(),
            architecture_summary: "商城 + AI 导购".to_string(),
            team_composition: vec!["backend".to_string(), "frontend".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐".to_string(),
            role_rationale: HashMap::from([(
                "backend".to_string(),
                "负责整体架构设计、API设计、数据库设计，确保双端一致性；同时负责AI导购问答能力的集成与实现（调用大模型API）。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "engineering-ai-engineer".to_string(),
                "AI集成职责已并入 backend 角色，由后端开发者调用大模型API实现导购问答。".to_string(),
            )]),
        }];
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.omittedRoleRationale 中 engineering-ai-engineer 的理由为“AI集成职责已并入 backend 角色”，但 backend 角色描述中未明确提及 AI 集成职责，仅在 roleRationale 中提及，需在 backend 角色描述中显式说明。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["backend 是否承接 AI？".to_string()],
            required_repairs: vec!["补充 backend AI 职责。".to_string()],
            review_summary: "误报。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_live_prebootstrap_clarification_blockers() {
        let mut session = base_session();
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "标准方案：完整电商体验+智能导购".to_string(),
            architecture_summary: "Web + 小程序 + 后端 API".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: "初始化协作包 LLM 预算约 10 万 token：需求/方案梳理约 3 万，星梦梦语义验收与修复约 3 万，目标软件协作包交接文档约 4 万；不包含业务代码生成或业务实现验收。".to_string(),
            recommendation_text: "推荐".to_string(),
            role_rationale: HashMap::from([(
                "docs".to_string(),
                "维护初始化协作包真源、接手文档和后续实施入口；Codex 后续接管时根入口为 AGENTS.md，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "marketing-content-creator".to_string(),
                "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色。".to_string(),
            )]),
        }];
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.omittedRoleRationale 中 'marketing-content-creator' 的解释 '该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色' 过于模糊，未说明具体并入哪个角色，违反真源规则要求角色取舍必须有依据。".to_string(),
                "selectedSolution.roleRationale 中 'docs' 角色描述提及 'Codex 后续接管时根入口为 AGENTS.md，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md'，但 targetClient 为 codex，这些入口文件在 pre_bootstrap 阶段尚未生成，属于预期行为，但描述中暗示了具体文件路径，可能造成后续阶段期望不一致，建议明确说明这些是规划中的文件结构而非已生成文件。".to_string(),
                "selectedSolution.tokenEstimate 中 '初始化协作包 LLM 预算约 10 万 token' 与 solutions 中方案 B 的 tokenEstimate 字段内容一致，但该字段在 selectedSolution 中重复出现，且未区分是方案本身的 token 预估还是协作包的 token 预算，可能引起混淆。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否修正？".to_string()],
            required_repairs: vec!["补充说明。".to_string()],
            review_summary: "live pre-bootstrap clarification blockers".to_string(),
            confidence: "high".to_string(),
        };
        let context = SemanticReviewContext::pre_bootstrap(&session);

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_prebootstrap_target_entry_path_promise_false_blocker() {
        let mut session = base_session();
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "前后端分离方案".to_string(),
            architecture_summary: "React + Spring Boot + MySQL".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "database".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: "初始化协作包 LLM 预算约 10 万 token；不包含业务代码生成或业务实现验收。"
                .to_string(),
            recommendation_text: "适合 Java 技术栈成熟的团队。".to_string(),
            role_rationale: HashMap::from([(
                "docs".to_string(),
                "维护初始化协作包真源、接手文档和后续实施入口；Codex 后续接管时根入口为 AGENTS.md，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "marketing-content-creator".to_string(),
                "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入 docs 角色，由文档维护人员负责内容创建。".to_string(),
            )]),
        }];
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.roleRationale 中 'docs' 角色描述提及 'Codex 后续接管时根入口为 AGENTS.md，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md'，但 reviewPhase 为 pre_bootstrap_solution_review，此时尚未生成文件，不应承诺具体文件路径，可能误导后续阶段。建议改为描述职责而非具体文件路径。".to_string(),
            ],
            questions_for_meng_xingxing: vec![
                "selectedSolution.roleRationale 中 'docs' 角色描述是否应避免提及具体文件路径？"
                    .to_string(),
            ],
            required_repairs: vec!["移除 Codex 入口路径说明。".to_string()],
            review_summary: "路径计划误报。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_prebootstrap_target_entry_plan_semantic_false_blocker() {
        let mut session = base_session();
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "标准企业方案".to_string(),
            architecture_summary: "React + Spring Boot + PostgreSQL".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "database".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "docs".to_string(),
                "维护初始化协作包真源、接手文档和后续实施入口；计划在后续阶段生成 AGENTS.md 作为 Codex 接管时的根入口，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "快速原型师".to_string(),
                "方案 B 采用成熟技术栈和标准开发流程，需求明确，无需快速原型验证。".to_string(),
            )]),
        }];
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.roleRationale.docs 中声称 'Codex 后续接管时根入口为 AGENTS.md，调度入口为 .codex/COORDINATOR-SUBAGENTS.md，角色文件位于 .codex/agents/*.md'，但当前 reviewPhase=pre_bootstrap_solution_review，这些文件尚未生成，且 roleRationale 应描述角色职责而非具体文件路径。该描述暗示文件已存在或作为业务运行环境，违反 targetClient 语义。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否应移除具体路径？".to_string()],
            required_repairs: vec!["修改 docs 职责描述。".to_string()],
            review_summary: "计划路径语义误报。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn package_roles_drop_unactivated_optional_miniapp_role() {
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "现代全栈派".to_string(),
            architecture_summary: "采用 Next.js 响应式 H5 作为移动端查询入口。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                (
                    "miniapp".to_string(),
                    "预留角色：若第一期形态为小程序，则负责微信小程序端页面；当前阶段为可选，待确认后启用。".to_string(),
                ),
                (
                    "frontend".to_string(),
                    "负责 Web 管理后台和移动端 H5 适配。".to_string(),
                ),
            ]),
            omitted_role_rationale: HashMap::new(),
        };

        let aligned = align_solution_with_package_roles(&solution, "balanced", Some("web-app"));

        assert!(!aligned.team_composition.contains(&"miniapp".to_string()));
        assert!(!aligned.role_rationale.contains_key("miniapp"));
        let miniapp_omission = aligned
            .omitted_role_rationale
            .get("miniapp")
            .expect("optional miniapp role should be recorded as omitted");
        assert!(miniapp_omission.contains("预留角色"));
        assert!(miniapp_omission.contains("未激活"));
    }

    #[test]
    fn normalize_rationale_propagates_omitted_role_assignments_to_owner_roles() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "低代码快速交付方案".to_string(),
            architecture_summary: "钉钉宜搭/飞书多维表格 + 自研前端".to_string(),
            team_composition: vec![
                "backend".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                ("backend".to_string(), "负责平台连接器。".to_string()),
                ("frontend".to_string(), "负责管理后台。".to_string()),
                ("miniapp".to_string(), "负责小程序端页面。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([
                (
                    "engineering-rapid-prototyper".to_string(),
                    "快速原型制作职责由 frontend 和 miniapp 角色在开发阶段通过低代码平台或组件库快速实现。".to_string(),
                ),
                (
                    "engineering-dingtalk-integration-developer".to_string(),
                    "钉钉集成开发职责由 backend 角色承担，负责对接钉钉开放平台 API。".to_string(),
                ),
                (
                    "engineering-feishu-integration-developer".to_string(),
                    "飞书集成开发职责由 backend 角色承担，负责对接飞书开放平台 API。".to_string(),
                ),
            ]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(solution
            .role_rationale
            .get("frontend")
            .unwrap()
            .contains("快速原型"));
        assert!(solution
            .role_rationale
            .get("miniapp")
            .unwrap()
            .contains("快速原型"));
        let backend_rationale = solution.role_rationale.get("backend").unwrap();
        assert!(backend_rationale.contains("钉钉"));
        assert!(backend_rationale.contains("飞书"));
    }

    #[test]
    fn normalize_rationale_does_not_assign_product_manager_to_qa_from_reviewer_acceptance() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统标准生产方案".to_string(),
            architecture_summary: "React + Spring Boot + MySQL。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "frontend".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                (
                    "architect".to_string(),
                    "把梦星星方案中的架构取舍沉淀为可执行实施边界。".to_string(),
                ),
                (
                    "qa".to_string(),
                    "制定测试计划，覆盖多角色权限边界、数据一致性、并发场景。".to_string(),
                ),
                (
                    "reviewer".to_string(),
                    "负责语义复核、风险检查和最终验收证据。".to_string(),
                ),
            ]),
            omitted_role_rationale: HashMap::from([(
                "产品经理".to_string(),
                "当前目标软件协作包不生成独立产品经理 Agent；后续迭代中的产品决策职责由 architect 兼任，reviewer 在验收时复核范围漂移。"
                    .to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(solution
            .role_rationale
            .get("architect")
            .unwrap()
            .contains("产品经理"));
        assert!(!solution
            .role_rationale
            .get("qa")
            .unwrap()
            .contains("产品经理"));
    }

    #[test]
    fn normalize_rationale_repairs_real_deepseek_low_code_semantic_loop_shape() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "跨端框架 + 轻量后端均衡方案".to_string(),
            architecture_summary: "低代码后台 + uni-app 小程序 + Web 双端。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "compliance".to_string(),
                "database".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: "初始化协作包 LLM 预算约 10 万 token：需求/方案梳理约 2 万，角色职责与边界定义约 1.5 万，星梦梦语义验收与修复约 2.5 万，目标软件协作包交接文档（含 AGENTS.md 和 .codex/ 模板）约 4 万；不包含业务代码生成或业务实现验收。".to_string(),
            recommendation_text: "推荐给需要双端体验一致、未来有扩展需求、不想被单一平台绑定的团队。".to_string(),
            role_rationale: HashMap::from([
                (
                    "architect".to_string(),
                    "负责低代码平台选型、平台边界、扩展点、权限/数据边界和后续退出方案，把梦星星方案沉淀为可执行实施边界。；承接 产品经理 的并入职责".to_string(),
                ),
                (
                    "backend".to_string(),
                    "负责低代码平台 API 集成、权限/业务规则配置、外部服务适配和必要的定制扩展边界。".to_string(),
                ),
                (
                    "compliance".to_string(),
                    "负责安全、合规、权限和敏感数据处理边界。".to_string(),
                ),
                (
                    "database".to_string(),
                    "负责低代码平台数据模型、字段关系、状态一致性、迁移策略、报表查询和关键统计边界。".to_string(),
                ),
                (
                    "docs".to_string(),
                    "维护初始化协作包真源、接手文档和后续实施入口；将在 bootstrap 阶段为目标软件（Codex）生成 AGENTS.md 和 .codex/ 入口文件，并记录交接路径。".to_string(),
                ),
                (
                    "frontend".to_string(),
                    "负责低代码平台管理后台页面配置、组件定制、表单/报表可见路径和 UI 一致性；不承接微信小程序端。".to_string(),
                ),
                (
                    "miniapp".to_string(),
                    "负责微信小程序端学生/家长查询入口、端侧状态、跨端交互和与低代码平台 API 的适配。".to_string(),
                ),
                (
                    "qa".to_string(),
                    "负责可执行测试计划、关键路径验证、技术回归、跨端一致性和缺陷证据；不替代 reviewer 的语义/范围复核。".to_string(),
                ),
                (
                    "reviewer".to_string(),
                    "负责语义复核、角色取舍、方案范围漂移和最终语义验收证据；不承接测试执行或跨端验证。".to_string(),
                ),
            ]),
            omitted_role_rationale: HashMap::from([
                (
                    "UI 设计师".to_string(),
                    "UI 设计职责并入 frontend 角色，由 frontend 负责低代码平台管理后台组件配置、视觉一致性和基础交互。".to_string(),
                ),
                (
                    "产品经理".to_string(),
                    "产品经理职责完全由 architect 角色承接，包括需求梳理、范围定义和产品决策；当前目标软件协作包不生成独立产品经理 Agent，后续迭代中 reviewer 在验收时复核范围漂移。".to_string(),
                ),
            ]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let architect = solution.role_rationale.get("architect").unwrap();
        assert!(!architect.contains("。；"));
        assert!(!architect.contains("承接 产品经理 的并入职责"));
        assert!(architect.contains("需求梳理"));
        assert!(architect.contains("用户故事优先级"));
        assert!(architect.contains("MVP 范围把控"));

        let docs = solution.role_rationale.get("docs").unwrap();
        assert!(docs.contains("当前负责维护初始化协作包真源"));
        assert!(docs.contains("实际落盘由启动器"));
        assert!(docs.contains("docs 负责记录交接说明"));
        assert!(!docs.contains("将在 bootstrap 阶段为目标软件"));

        let ui_omission = solution.omitted_role_rationale.get("UI 设计师").unwrap();
        assert!(ui_omission.contains("frontend"));
        assert!(ui_omission.contains("miniapp"));
        assert!(ui_omission.contains("小程序端"));

        let reviewer = solution.role_rationale.get("reviewer").unwrap();
        assert!(reviewer.contains("目标软件协作包内"));
        assert!(reviewer.contains("星梦梦"));
        assert!(reviewer.contains("阶段不同"));

        assert!(!solution.token_estimate.contains("约 10 万"));
        assert!(solution.token_estimate.contains("14-18 万"));
        assert!(solution.token_estimate.contains("9 个角色"));
    }

    #[test]
    fn normalize_rationale_marks_mobile_delivery_as_assumption_when_still_pending() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统标准生产方案".to_string(),
            architecture_summary: "前端Vue3管理后台+移动端H5，后端Spring Boot。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "miniapp".to_string(),
                "负责移动端H5页面、端侧状态和跨端交互边界。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "UX研究员".to_string(),
                "移动端形式待确认，当前阶段由 architect 和 frontend 进行初步调研，后续根据需求决定是否引入。"
                    .to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(solution.architecture_summary.contains("移动端形式待确认"));
        assert!(solution
            .role_rationale
            .get("miniapp")
            .unwrap()
            .contains("移动端形式待确认"));
    }

    #[test]
    fn default_docs_role_rationale_marks_codex_entries_as_planned_bootstrap_files() {
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统标准生产方案".to_string(),
            architecture_summary: "React + Spring Boot。".to_string(),
            team_composition: vec!["docs".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::new(),
            omitted_role_rationale: HashMap::new(),
        };

        let rationale = default_package_role_rationale("docs", &solution);

        assert!(rationale.contains("计划在 bootstrap 阶段生成"));
        assert!(rationale.contains("AGENTS.md"));
        assert!(rationale.contains(".codex"));
        assert!(rationale.contains("目标软件"));
        assert!(rationale.contains("接管入口文件"));
        assert!(!rationale.contains("负责在 bootstrap 阶段生成"));
        assert!(!rationale.contains("已生成"));
        assert!(!rationale.contains("已存在"));
        assert!(!rationale.contains(".codex/COORDINATOR-SUBAGENTS.md"));
    }

    #[test]
    fn normalize_rationale_keeps_repaired_docs_codex_entry_commitment() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统标准生产方案".to_string(),
            architecture_summary: "React + Spring Boot。".to_string(),
            team_composition: vec!["docs".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "docs".to_string(),
                "计划在 bootstrap 阶段生成并维护 Codex 接管入口 AGENTS.md 和 .codex 文件，并由 postcheck 验证；当前阶段不声称文件已存在。"
                    .to_string(),
            )]),
            omitted_role_rationale: HashMap::new(),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let docs = solution
            .role_rationale
            .get("docs")
            .expect("docs rationale should remain present");
        assert!(docs.contains("AGENTS.md"));
        assert!(docs.contains(".codex"));
        assert!(docs.contains("计划在 bootstrap 阶段生成"));
        assert!(!docs.contains("已生成"));
    }

    #[test]
    fn normalize_rationale_does_not_turn_security_omission_into_ui_ux_assignment() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统轻量原型".to_string(),
            architecture_summary: "Vue 3 + FastAPI + SQLite。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "frontend".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                ("architect".to_string(), "负责权限边界设计。".to_string()),
                ("backend".to_string(), "负责认证授权实现。".to_string()),
                ("frontend".to_string(), "负责页面体验。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([(
                "安全工程师".to_string(),
                "安全职责由 architect 和 backend 分担，architect 负责安全合规性评估和权限边界设计，backend 负责认证授权和输入校验。"
                    .to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let backend = solution
            .role_rationale
            .get("backend")
            .expect("backend rationale should be present");
        assert!(backend.contains("安全"));
        assert!(!backend.contains("UI/UX"));
    }

    #[test]
    fn align_solution_records_product_manager_omission_even_when_source_does_not_mention_it() {
        let solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统轻量原型".to_string(),
            architecture_summary: "Vue 3 + FastAPI + SQLite。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "frontend".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::new(),
            omitted_role_rationale: HashMap::new(),
        };

        let aligned = align_solution_with_package_roles(&solution, "balanced", Some("web-app"));

        let omission = aligned
            .omitted_role_rationale
            .get("产品经理")
            .expect("product manager omission should be explicit for every generated package");
        assert!(omission.contains("产品决策"));
        assert!(omission.contains("architect"));
    }

    #[test]
    fn normalize_rationale_aligns_product_manager_omission_with_reviewer_scope_drift() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "学生管理系统轻量原型".to_string(),
            architecture_summary: "Vue 3 + FastAPI + SQLite。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "frontend".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "reviewer".to_string(),
                "负责语义复核、风险检查和最终验收证据。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "产品经理".to_string(),
                "当前目标软件协作包不生成独立产品经理 Agent；后续迭代中的产品决策职责由 architect 兼任，reviewer 在验收时复核范围漂移。"
                    .to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(solution
            .role_rationale
            .get("reviewer")
            .unwrap()
            .contains("范围漂移"));
        assert!(solution
            .role_rationale
            .get("qa")
            .unwrap()
            .contains("技术回归"));
    }

    #[test]
    fn normalize_rationale_separates_reviewer_and_qa_scopes() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "浴霸网站低代码方案".to_string(),
            architecture_summary: "低代码后台 + 小程序 + Web 双端。".to_string(),
            team_composition: vec!["reviewer".to_string(), "qa".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                (
                    "reviewer".to_string(),
                    "与 qa 分工为语义/范围复核，而非替代执行回归。".to_string(),
                ),
                ("qa".to_string(), "确保架构与技术选型回归证据。".to_string()),
            ]),
            omitted_role_rationale: HashMap::new(),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let reviewer = solution.role_rationale.get("reviewer").unwrap();
        let qa = solution.role_rationale.get("qa").unwrap();
        assert!(reviewer.contains("语义复核"));
        assert!(reviewer.contains("范围漂移"));
        assert!(!reviewer.contains("技术回归"));
        assert!(qa.contains("技术回归"));
        assert!(qa.contains("关键路径"));
        assert!(!qa.contains("角色取舍"));
    }

    #[test]
    fn normalize_rationale_expands_ai_engineer_omission_into_backend_rag_scope() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "浴霸导购网站方案".to_string(),
            architecture_summary:
                "AI导购直接接入百度文心一言或通义千问的官方API，并使用知识库平台承载产品问答。"
                    .to_string(),
            team_composition: vec!["backend".to_string(), "reviewer".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "backend".to_string(),
                "负责AI导购API集成（百度文心一言/通义千问）及知识库平台对接。"
                    .to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "AI工程师".to_string(),
                "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入 backend 角色（负责AI API集成）"
                    .to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let backend = solution.role_rationale.get("backend").unwrap();
        let ai_omission = solution.omitted_role_rationale.get("AI工程师").unwrap();
        assert!(backend.contains("RAG") || backend.contains("知识库搭建"));
        assert!(backend.contains("模型参数") || backend.contains("提示词"));
        assert!(ai_omission.contains("RAG") || ai_omission.contains("知识库搭建"));
        assert!(ai_omission.contains("后续复杂模型调优"));
    }

    #[test]
    fn normalize_rationale_aligns_security_platform_omission_with_architect_and_backend() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "浴霸网站方案".to_string(),
            architecture_summary: "Supabase + 支付 SDK + AI 导购。".to_string(),
            team_composition: vec!["architect".to_string(), "backend".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                (
                    "architect".to_string(),
                    "承担安全合规性评估和权限边界设计职责。".to_string(),
                ),
                ("backend".to_string(), "负责接口集成。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([(
                "安全工程师".to_string(),
                "使用成熟平台，基础安全由服务商保障。".to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let omission = solution.omitted_role_rationale.get("安全工程师").unwrap();
        let backend = solution.role_rationale.get("backend").unwrap();
        assert!(omission.contains("architect"));
        assert!(omission.contains("backend"));
        assert!(!omission.contains("基础安全由服务商保障。"));
        assert!(backend.contains("认证授权"));
    }

    #[test]
    fn semantic_review_sanitizer_drops_business_dependency_capability_false_blockers() {
        let session = base_session();
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.architectureSummary 中 'AI导购直接接入百度文心一言或通义千问的官方API，使用零代码知识库平台'，但 selectedCapabilities 中未包含任何与 AI 或知识库平台相关的能力（如 'baidu-knowledge-base' 或 'tongyi-api'），能力清单与架构方案矛盾。".to_string(),
                "selectedSolution.architectureSummary 中 '支付使用微信支付+支付宝官方SDK'，但 selectedCapabilities 中未包含支付相关能力（如 'wechat-pay' 或 'alipay'），能力清单与架构方案矛盾。".to_string(),
                "selectedSolution.architectureSummary 中 '数据库使用Supabase（免费额度足够）'，但 selectedCapabilities 中未包含数据库相关能力（如 'supabase'），能力清单与架构方案矛盾。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否补能力？".to_string()],
            required_repairs: vec!["补能力。".to_string()],
            review_summary: "业务依赖被误判为启动器能力。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_consistency_and_acceptability_observations() {
        let session = base_session();
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.omittedRoleRationale 中 '快速原型师' 的省略理由为 '快速原型职责由 architect 承担'，但 architect 的 roleRationale 中已明确 '承接快速原型制作职责'，两者一致，无矛盾。".to_string(),
                "selectedSolution.roleRationale 中 docs 角色职责中列出的具体文件路径可能过于具体，但作为计划允许。".to_string(),
                "selectedSolution.omittedRoleRationale 中 '安全工程师' 的省略理由为 '原型阶段 JWT 基本保护即可'，但 architectureSummary 中使用了 Casbin，可能涉及更复杂的权限模型，但原型阶段可接受。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否需要处理？".to_string()],
            required_repairs: vec!["请处理。".to_string()],
            review_summary: "非阻断观察被放进阻断项。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn normalize_rationale_aligns_security_omission_with_architect_assessment() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "低代码+定制方案".to_string(),
            architecture_summary: "Mendix 低代码平台 + 微信小程序。".to_string(),
            team_composition: vec!["architect".to_string(), "backend".to_string()],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([(
                "architect".to_string(),
                "负责低代码平台选型、平台边界和扩展点。".to_string(),
            )]),
            omitted_role_rationale: HashMap::from([(
                "安全工程师".to_string(),
                "Mendix 平台内置安全功能，但需评估合规性。".to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        assert!(solution
            .role_rationale
            .get("architect")
            .unwrap()
            .contains("安全合规"));
    }

    #[test]
    fn normalize_rationale_specializes_low_code_solution_roles() {
        let mut solution = AgentSolution {
            id: "B".to_string(),
            title: "低代码+定制方案（Mendix + 微信小程序）".to_string(),
            architecture_summary:
                "使用低代码平台Mendix快速搭建管理后台，微信小程序作为移动端查询入口。".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "database".to_string(),
                "frontend".to_string(),
                "miniapp".to_string(),
                "qa".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "适合快速上线。".to_string(),
            role_rationale: HashMap::from([
                (
                    "frontend".to_string(),
                    "负责 Web 管理后台和移动端 H5。".to_string(),
                ),
                (
                    "backend".to_string(),
                    "设计低代码平台上的数据模型和业务逻辑。".to_string(),
                ),
                (
                    "database".to_string(),
                    "负责数据模型、状态一致性、迁移和关键查询边界。".to_string(),
                ),
                (
                    "qa".to_string(),
                    "测试低代码平台生成的业务逻辑正确性。".to_string(),
                ),
                (
                    "miniapp".to_string(),
                    "负责微信小程序端页面、端侧状态和跨端交互边界。".to_string(),
                ),
            ]),
            omitted_role_rationale: HashMap::from([(
                "移动应用开发者".to_string(),
                "小程序由前端开发者承担".to_string(),
            )]),
        };

        normalize_repaired_solution_rationale(&mut solution);

        let frontend = solution.role_rationale.get("frontend").unwrap();
        assert!(frontend.contains("低代码平台管理后台"));
        assert!(!frontend.contains("移动端 H5"));
        assert!(solution
            .role_rationale
            .get("backend")
            .unwrap()
            .contains("API"));
        assert!(solution
            .role_rationale
            .get("database")
            .unwrap()
            .contains("低代码平台数据模型"));
        assert!(solution
            .omitted_role_rationale
            .get("移动应用开发者")
            .unwrap()
            .contains("miniapp"));
    }

    #[test]
    fn semantic_review_sanitizer_respects_project_name_and_postcheck_evidence() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec![
                "AGENTS.md".to_string(),
                ".codex/COORDINATOR-SUBAGENTS.md".to_string(),
                ".codex/agents/engineering-frontend-developer.md".to_string(),
            ],
            generated_file_evidence: vec![
                GeneratedFileEvidence {
                    path: "AGENTS.md".to_string(),
                    relative_path: "AGENTS.md".to_string(),
                    content_preview: "默认真源入口\n4. `.codex/COORDINATOR-SUBAGENTS.md`"
                        .to_string(),
                },
                GeneratedFileEvidence {
                    path: ".codex/COORDINATOR-SUBAGENTS.md".to_string(),
                    relative_path: ".codex/COORDINATOR-SUBAGENTS.md".to_string(),
                    content_preview: "# 主控调度指南".to_string(),
                },
            ],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "真源规则要求产品主名称必须是'星星的vibecoding启动器'，但所有文档中均使用'校园社团协作包'，未出现该名称。".to_string(),
                "targetClient为codex，但AGENTS.md中未包含.codex入口文件路径或明确指向.codex目录的引用。".to_string(),
                "docs/architecture/01-项目架构设计书.md中大量使用模板腔调，未提供具体技术栈或架构细节。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否需要全局替换项目名？".to_string()],
            required_repairs: vec!["将项目名替换为启动器名。".to_string()],
            review_summary: "误报。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_redundant_codex_entry_confirmation() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec![
                "AGENTS.md".to_string(),
                ".codex/COORDINATOR-SUBAGENTS.md".to_string(),
            ],
            generated_file_evidence: vec![GeneratedFileEvidence {
                path: "AGENTS.md".to_string(),
                relative_path: "AGENTS.md".to_string(),
                content_preview:
                    "后续请在 Codex 中从 `AGENTS.md` 重新接手。\n本文件是 Codex 的原生入口。\n默认真源入口：`.codex/COORDINATOR-SUBAGENTS.md`"
                        .to_string(),
            }],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "targetClient为codex，但生成的AGENTS.md中明确要求'在Codex中从AGENTS.md重新接手'，而AGENTS.md本身是入口文件，但内容中未明确说明AGENTS.md是Codex的原生入口文件，且AGENTS.md中写的是'本文件是Codex的原生入口'，但未明确说明这是targetClient要求的入口文件。实际上AGENTS.md已存在且内容正确，但需确认是否满足targetClient要求。".to_string(),
                "generatedFileEvidence中AGENTS.md的内容提到'本文件是Codex的原生入口'，但未明确说明这是targetClient要求的入口文件。需确认AGENTS.md是否明确作为Codex的入口文件。".to_string(),
            ],
            questions_for_meng_xingxing: vec![
                "产品主名称应为'星星的vibecoding启动器'，但所有文档中均使用'浴霸电商协作包'，是否需要全局替换？".to_string(),
                "AGENTS.md中已写明'本文件是Codex的原生入口'，是否满足targetClient=codex的入口文件要求？是否需要更明确的声明？".to_string(),
            ],
            required_repairs: vec![
                "将所有文档中的'浴霸电商协作包'替换为'星星的vibecoding启动器'，包括文件名、内容中的项目名称。".to_string(),
                "在AGENTS.md中明确声明'本文件是Codex的原生入口文件，后续接管软件为Codex'，以符合targetClient要求。".to_string(),
            ],
            review_summary: "入口误报。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_self_declared_non_blockers() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec!["AGENTS.md".to_string()],
            generated_file_evidence: vec![GeneratedFileEvidence {
                path: "AGENTS.md".to_string(),
                relative_path: "AGENTS.md".to_string(),
                content_preview: "本文件是 Codex 的原生入口。".to_string(),
            }],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "capabilityState 中所有能力状态为 fallback，规则允许 fallback 不阻断。".to_string(),
                "reviewPhase为final_generated_package_review，generatedFileEvidence非空，且包含AGENTS.md等入口文件，符合阶段要求。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否阻断？".to_string()],
            required_repairs: vec!["不需要修复。".to_string()],
            review_summary: "自相矛盾的阻断项。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_handles_passed_review_with_nonblocking_issue_text() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec!["AGENTS.md".to_string(), ".specify/spec.md".to_string()],
            generated_file_evidence: vec![GeneratedFileEvidence {
                path: "AGENTS.md".to_string(),
                relative_path: "AGENTS.md".to_string(),
                content_preview: "本文件是 Codex 的原生入口。".to_string(),
            }],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: true,
            blocking_issues: vec![
                "selectedSolution 提到支付和存储业务依赖，但 selectedCapabilities 是启动器/协作包能力清单，不阻塞。".to_string(),
                "generatedFiles 中包含 .specify/，Speckit 能力已选择，生成 .specify/ 是合理的，不阻塞。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否需要补充说明？".to_string()],
            required_repairs: vec![],
            review_summary: "通过，但模型把非阻塞说明误放进 blockingIssues。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_self_described_no_issue_blocker() {
        let session = base_session();
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution.recommendationText 中提及“本方案为“星星的vibecoding启动器”的初始化协作包”，但产品主名称规则要求启动器自身对外名称固定为“星星的vibecoding启动器”，此处使用正确，无问题。但需确认用户原始需求中是否要求产品主名称替换，若未要求，则无阻断。".to_string(),
            ],
            questions_for_meng_xingxing: vec![],
            required_repairs: vec![],
            review_summary: "星梦梦返回 passed=true 但 blockingIssues 非空，程序按阻断处理并要求回传梦星星修正或说明。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_explicit_no_problem_observations() {
        let session = base_session();
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "selectedSolution 的 architectureSummary 和 recommendationText 与用户需求一致，未发现遗漏或矛盾。".to_string(),
                "三方案完整性：solutions 包含三个方案（A、B、C），且 selectedSolution 为 B，未破坏完整性。".to_string(),
                "未发现模板腔调或角色取舍无依据的问题。".to_string(),
            ],
            questions_for_meng_xingxing: vec![],
            required_repairs: vec![],
            review_summary: "星梦梦返回 passed=true 但 blockingIssues 非空，程序按阻断处理并要求回传梦星星修正或说明。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_explicit_rule_compliant_observations() {
        let session = base_session();
        let context = SemanticReviewContext::pre_bootstrap(&session);
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "generatedFileEvidence 为空是 pre_bootstrap 阶段的预期状态，不能因此阻断。".to_string(),
                "targetClient 为 codex，其含义正确解释为 'AGENTS.md/.codex handoff'，且未要求业务方案运行在 codex 中，符合规则。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否确认？".to_string()],
            required_repairs: vec![],
            review_summary: "非阻断合规观察。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_preview_truncation_when_file_exists_and_postcheck_passed() {
        let root = repo_tmp_dir("semantic-preview-truncation");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".specify")).unwrap();
        fs::write(
            root.join(".specify").join("extensions.yml"),
            "installed: []\nsettings:\n  auto_execute_hooks: true\n",
        )
        .unwrap();
        let mut session = base_session();
        session.workspace_path = root.to_string_lossy().to_string();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec![".specify/extensions.yml".to_string()],
            generated_file_evidence: vec![GeneratedFileEvidence {
                path: root
                    .join(".specify")
                    .join("extensions.yml")
                    .to_string_lossy()
                    .to_string(),
                relative_path: ".specify/extensions.yml".to_string(),
                content_preview: "installed: []\nsettings:\n  auto_execute_hooks: true\n..."
                    .to_string(),
            }],
            postcheck_passed: Some(true),
            truth_source_rules: vec!["truth-source gate passed".to_string()],
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "generatedFileEvidence 中 .specify/extensions.yml 的 contentPreview 被截断，无法确认文件完整性。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["请确认文件完整性。".to_string()],
            required_repairs: vec!["请检查 .specify/extensions.yml 文件是否完整。".to_string()],
            review_summary: "预览截断误报。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_drops_generic_missing_role_rationale_when_present() {
        let mut session = base_session();
        session.selected_solution_id = Some("B".to_string());
        session.solutions = vec![AgentSolution {
            id: "B".to_string(),
            title: "Next.js 全栈一体化方案".to_string(),
            architecture_summary: "Next.js + Prisma + PostgreSQL".to_string(),
            team_composition: vec![
                "architect".to_string(),
                "backend".to_string(),
                "compliance".to_string(),
                "devops".to_string(),
                "docs".to_string(),
                "frontend".to_string(),
                "qa".to_string(),
                "reviewer".to_string(),
            ],
            token_estimate: default_package_token_estimate(),
            recommendation_text: "推荐方案。".to_string(),
            role_rationale: HashMap::from([
                ("architect".to_string(), "沉淀架构取舍。".to_string()),
                ("backend".to_string(), "负责接口和数据真源。".to_string()),
                ("compliance".to_string(), "负责权限和敏感数据边界。".to_string()),
                ("devops".to_string(), "负责部署和环境变量。".to_string()),
                ("docs".to_string(), "维护接手文档。".to_string()),
                ("frontend".to_string(), "负责管理后台和移动 H5。".to_string()),
                ("qa".to_string(), "负责关键路径验证。".to_string()),
                ("reviewer".to_string(), "负责语义复核。".to_string()),
            ]),
            omitted_role_rationale: HashMap::from([
                (
                    "UI 设计师 ×0.5".to_string(),
                    "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色。".to_string(),
                ),
                (
                    "全栈开发者 ×2-3（需熟悉 React/Next.js）".to_string(),
                    "该自然语言角色未映射为当前目标软件协作包的独立执行 Agent；相关职责已并入标准协作角色。".to_string(),
                ),
            ]),
        }];
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec!["AGENTS.md".to_string()],
            generated_file_evidence: vec![],
            postcheck_passed: Some(true),
            truth_source_rules: vec!["postcheck passed".to_string()],
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec!["梦星星没有说明角色选择理由和明显候选角色不选理由。".to_string()],
            questions_for_meng_xingxing: vec![
                "请逐条说明当前团队中每个角色为什么需要。".to_string()
            ],
            required_repairs: vec![
                "补齐 roleRationale 与 omittedRoleRationale 后重新交给星梦梦复核。".to_string(),
            ],
            review_summary: "泛化角色理由误报。".to_string(),
            confidence: "high".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn semantic_review_sanitizer_forces_failure_when_passed_review_keeps_real_blocker() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec!["AGENTS.md".to_string()],
            generated_file_evidence: vec![GeneratedFileEvidence {
                path: "AGENTS.md".to_string(),
                relative_path: "AGENTS.md".to_string(),
                content_preview: "业务工作流与方案不一致。".to_string(),
            }],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: true,
            blocking_issues: vec![
                "AGENTS.md 的业务协作工作流与 selectedSolution 明显矛盾，必须修复。".to_string(),
            ],
            questions_for_meng_xingxing: vec![],
            required_repairs: vec!["修复 AGENTS.md 工作流。".to_string()],
            review_summary: "自相矛盾地声明通过。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(!sanitized.passed);
        assert_eq!(sanitized.blocking_issues.len(), 1);
        assert!(sanitized.review_summary.contains("blockingIssues"));
    }

    #[test]
    fn semantic_review_sanitizer_drops_postcheck_passed_template_advice() {
        let session = base_session();
        let context = SemanticReviewContext {
            phase: "final_generated_package_review".to_string(),
            workspace_path: session.workspace_path.clone(),
            generated_files: vec![
                "docs/workflow/first-sprint-contract.md".to_string(),
                "docs/workflow/sprint-contract-template.md".to_string(),
                "docs/agents/reviewer-handbook.md".to_string(),
            ],
            generated_file_evidence: vec![],
            postcheck_passed: Some(true),
            truth_source_rules: semantic_truth_source_rules(),
        };
        let review = SemanticReviewResult {
            passed: false,
            blocking_issues: vec![
                "docs/workflow/sprint-contract-template.md 和 first-sprint-contract.md 内容几乎完全重复，且 first-sprint-contract.md 中交付物清单与当前初始化协作包阶段不符，应考虑删除。".to_string(),
                "docs/agents/reviewer-handbook.md 中 '评估者模式' 和 '最终门禁模式' 描述详细，但当前阶段为初始化协作包，尚未进入实施，这些文档可能过早引入复杂性。".to_string(),
            ],
            questions_for_meng_xingxing: vec!["是否删除模板？".to_string()],
            required_repairs: vec!["删除过早文档。".to_string()],
            review_summary: "postcheck 后的模板建议。".to_string(),
            confidence: "medium".to_string(),
        };

        let sanitized = sanitize_semantic_review_against_session(&session, review, &context);

        assert!(sanitized.passed);
        assert!(sanitized.blocking_issues.is_empty());
        assert!(sanitized.questions_for_meng_xingxing.is_empty());
        assert!(sanitized.required_repairs.is_empty());
    }

    #[test]
    fn generated_file_evidence_prioritizes_target_client_entry_files() {
        let root = repo_tmp_dir("evidence-priority");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".specify").join("templates")).unwrap();
        fs::create_dir_all(root.join(".codex")).unwrap();
        for index in 0..24 {
            fs::write(
                root.join(".specify")
                    .join("templates")
                    .join(format!("template-{index}.md")),
                format!("# Template {index}"),
            )
            .unwrap();
        }
        fs::write(root.join("AGENTS.md"), "# AGENTS\n星星的vibecoding启动器").unwrap();
        fs::write(
            root.join(".codex").join("COORDINATOR-SUBAGENTS.md"),
            "# Coordinator",
        )
        .unwrap();
        let mut generated_files = (0..24)
            .map(|index| {
                root.join(".specify")
                    .join("templates")
                    .join(format!("template-{index}.md"))
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        generated_files.push(root.join("AGENTS.md").to_string_lossy().to_string());
        generated_files.push(
            root.join(".codex")
                .join("COORDINATOR-SUBAGENTS.md")
                .to_string_lossy()
                .to_string(),
        );

        let evidence = collect_generated_file_evidence(&root.to_string_lossy(), &generated_files);

        assert!(evidence
            .iter()
            .any(|item| item.relative_path.ends_with("AGENTS.md")));
        assert!(evidence
            .iter()
            .any(|item| item.relative_path.contains("COORDINATOR-SUBAGENTS.md")));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    #[ignore = "requires live MiniMax API access"]
    fn live_custom_minimax_agent_decision_smoke() {
        let api_key = std::env::var("COMMONHE_MINIMAX_TEST_KEY")
            .expect("COMMONHE_MINIMAX_TEST_KEY must be set for live MiniMax tests");
        let model = std::env::var("COMMONHE_MINIMAX_TEST_MODEL")
            .unwrap_or_else(|_| "MiniMax-M2.7".to_string());
        let validation = provider::validate_provider_connection(&ProviderConfig {
            provider: "custom".to_string(),
            model: Some(model.clone()),
            api_key: Some(api_key.clone()),
            base_url: Some("https://api.minimaxi.com".to_string()),
        });

        assert!(
            validation.valid,
            "{}",
            validation
                .user_facing_error
                .clone()
                .unwrap_or_else(|| validation.errors.join(","))
        );
        assert_eq!(validation.resolved_wire_api, "chat_completions");
        assert_eq!(
            validation.resolved_base_url.as_deref(),
            Some("https://api.minimaxi.com/v1")
        );

        let mut session = base_session();
        session.provider = "custom".to_string();
        session.model = validation.resolved_model.unwrap_or(model);
        session.api_key = api_key;
        session.base_url = validation
            .resolved_base_url
            .expect("MiniMax validation should resolve /v1 base URL");
        session.wire_api = validation.resolved_wire_api;

        let decision = request_agent_decision(&session, Some("session_start"))
            .expect("live MiniMax custom provider should return a parseable agent decision");

        assert_eq!(decision.mode, "question");
        assert!(!decision.assistant_message.trim().is_empty());
    }

    #[test]
    #[ignore = "requires live MiniMax API access"]
    fn live_custom_minimax_ecommerce_reaches_three_solutions() {
        let api_key = std::env::var("COMMONHE_MINIMAX_TEST_KEY")
            .expect("COMMONHE_MINIMAX_TEST_KEY must be set for live MiniMax tests");
        let model = std::env::var("COMMONHE_MINIMAX_TEST_MODEL")
            .unwrap_or_else(|_| "MiniMax-M2.7".to_string());
        let workspace = repo_tmp_dir("live-custom-minimax-ecommerce-solutions");
        let store = AgentStore::new();
        let repo_root = repo_root();
        let request = AgentSessionCreateRequest {
            provider: "custom".to_string(),
            model,
            api_key,
            base_url: "https://api.minimaxi.com/v1".to_string(),
            wire_api: "chat_completions".to_string(),
            workspace_path: workspace.to_string_lossy().to_string(),
            payload_root: repo_root.clone(),
            orchestrator_path: repo_root
                .join("tools")
                .join("common-he-init-orchestrator.ps1"),
        };

        let initial = store
            .create_session(request)
            .expect("live MiniMax ecommerce session should start");
        assert_eq!(initial.stage, "conversation");

        let after_product = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "B2B 批发电商网站。".to_string(),
            })
            .expect("MiniMax product type prompt should succeed");
        assert!(after_product.solutions.is_empty());

        let mut after_detail = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "面向浴室浴霸行业中小批发商、经销商、工程采购商；核心功能包括商品展示、下单支付、用户评价、搜索过滤、优惠券系统、直播带货、后台管理。技术要求是 SaaS 方案、快速上线。请先总结你的理解，不要直接给方案。".to_string(),
            })
            .expect("MiniMax ecommerce detail prompt should succeed");
        assert!(after_detail.solutions.is_empty());

        if !after_detail.readiness.summary_presented {
            after_detail = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: "请先总结你的理解，我确认后你再给三个方案。".to_string(),
                })
                .expect("MiniMax summary prompt should succeed");
            assert!(after_detail.solutions.is_empty());
        }

        let after_confirm = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "准确。".to_string(),
            })
            .expect("MiniMax confirmation should produce or request three solutions without transport failure");

        assert_eq!(
            after_confirm.solutions.len(),
            3,
            "{}",
            after_confirm
                .messages
                .last()
                .map(|message| message.content.clone())
                .unwrap_or_else(|| "missing assistant reply".to_string())
        );
        assert!(after_confirm.tool_calls.iter().any(|tool_call| {
            tool_call.tool_name == "open_solution_selector" && tool_call.status == "requested"
        }));
    }

    #[test]
    #[ignore = "requires live DeepSeek API access"]
    fn live_deepseek_agent_flow_smoke() {
        let api_key = std::env::var("COMMONHE_DEEPSEEK_TEST_KEY")
            .expect("COMMONHE_DEEPSEEK_TEST_KEY must be set for live DeepSeek tests");
        run_live_agent_flow(
            "deepseek",
            "deepseek-v4-flash".to_string(),
            api_key,
            "https://api.deepseek.com".to_string(),
        );
    }

    #[test]
    #[ignore = "requires live DeepSeek API access and writes the requested target workspace"]
    fn live_deepseek_ecommerce_agent_flow_lands_to_requested_workspace() {
        let api_key = std::env::var("COMMONHE_DEEPSEEK_TEST_KEY")
            .expect("COMMONHE_DEEPSEEK_TEST_KEY must be set for live DeepSeek tests");
        let workspace = std::env::var("COMMONHE_LIVE_TARGET_WORKSPACE")
            .unwrap_or_else(|_| "E:\\test\\test-shop".to_string());
        run_live_ecommerce_agent_flow_to_workspace(
            "deepseek",
            "deepseek-v4-flash".to_string(),
            api_key,
            "https://api.deepseek.com".to_string(),
            PathBuf::from(workspace),
        );
    }

    #[test]
    #[ignore = "requires live DeepSeek API access and writes the requested target workspace"]
    fn live_deepseek_student_management_agent_flow_lands_to_requested_workspace() {
        let api_key = std::env::var("COMMONHE_DEEPSEEK_TEST_KEY")
            .expect("COMMONHE_DEEPSEEK_TEST_KEY must be set for live DeepSeek tests");
        let workspace = std::env::var("COMMONHE_LIVE_STUDENT_TARGET_WORKSPACE")
            .unwrap_or_else(|_| "E:\\test\\student-management-system".to_string());
        run_live_student_management_agent_flow_to_workspace(
            "deepseek",
            "deepseek-v4-flash".to_string(),
            api_key,
            "https://api.deepseek.com".to_string(),
            PathBuf::from(workspace),
        );
    }

    #[test]
    #[ignore = "requires local Codex auth.json with valid OpenAI credentials"]
    fn live_codex_agent_flow_from_local_auth_smoke() {
        let validation = provider::validate_provider_connection(&ProviderConfig {
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

        let api_key = provider::resolved_api_key_for_provider("codex", None)
            .expect("Codex auth source should yield an API key for live tests");

        run_live_agent_flow(
            "codex",
            validation
                .resolved_model
                .clone()
                .unwrap_or_else(|| "gpt-5.4".to_string()),
            api_key,
            validation
                .resolved_base_url
                .clone()
                .unwrap_or_else(|| "https://api.openai.com/v1".to_string()),
        );
    }

    fn run_live_agent_flow(provider_name: &str, model: String, api_key: String, base_url: String) {
        let store = AgentStore::new();
        let repo_root = repo_root();
        let workspace = repo_tmp_dir(&format!("live-agent-{provider_name}"));
        let request = AgentSessionCreateRequest {
            provider: provider_name.to_string(),
            model,
            api_key,
            base_url,
            wire_api: provider::resolved_wire_api_for_provider(provider_name, None),
            workspace_path: workspace.to_string_lossy().to_string(),
            payload_root: repo_root.clone(),
            orchestrator_path: repo_root
                .join("tools")
                .join("common-he-init-orchestrator.ps1"),
        };

        let initial = store
            .create_session(request)
            .expect("live session should start");
        assert_eq!(initial.stage, "conversation");
        assert!(initial.solutions.is_empty());

        let after_vague = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "我想做一个给校园社团用的网站。".to_string(),
            })
            .expect("vague live prompt should succeed");
        assert!(after_vague.solutions.is_empty());

        let mut after_detail = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "目标用户是大学里的社团负责人和新成员。核心问题是活动报名、通知和资料分发很分散。关键功能需要活动发布、报名、资料库和消息通知。约束是先做网页版本，优先一周内跑通 MVP。请先总结你的理解，不要直接给方案。".to_string(),
            })
            .expect("detail live prompt should succeed");
        assert!(after_detail.solutions.is_empty());

        if !after_detail.readiness.summary_presented {
            after_detail = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: "请先总结你的理解，我确认后你再给三个方案。".to_string(),
                })
                .expect("summary prompt should succeed");
            assert!(after_detail.solutions.is_empty());
        }

        let mut after_confirm = after_detail.clone();
        for follow_up in [
            "是的，按这个理解，请给我三个方案。",
            "是的，这个总结准确，请直接给我三个方案，每个方案都包含架构、团队组成和 token 预估。",
            "如果你还在等待确认，那么我现在明确确认：你的总结准确。请直接给我三个方案。",
        ] {
            after_confirm = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: follow_up.to_string(),
                })
                .expect("solution follow-up prompt should succeed");

            if after_confirm.solutions.len() == 3 {
                break;
            }
        }

        assert_eq!(
            after_confirm.solutions.len(),
            3,
            "{}",
            after_confirm
                .messages
                .last()
                .map(|message| message.content.clone())
                .unwrap_or_else(|| "missing assistant reply".to_string())
        );

        let bootstrap_session = store
            .start_solution_bootstrap(AgentChooseRequest {
                session_id: initial.session_id.clone(),
                solution_id: after_confirm.solutions[0].id.clone(),
                project_name: "校园社团协作包".to_string(),
                target_client: "codex".to_string(),
                selected_capabilities: vec![],
            })
            .expect("solution bootstrap should start");
        let bootstrap_result = execute_solution_bootstrap(&bootstrap_session)
            .expect("bootstrap should succeed after live solutions");
        assert_eq!(bootstrap_result.status, "success");
        assert!(bootstrap_result.postcheck_passed);
        assert!(!bootstrap_result.generated_files.is_empty());
        let _ = fs::remove_dir_all(&workspace);
    }

    fn run_live_ecommerce_agent_flow_to_workspace(
        provider_name: &str,
        model: String,
        api_key: String,
        base_url: String,
        workspace: PathBuf,
    ) {
        let store = AgentStore::new();
        let repo_root = repo_root();
        fs::create_dir_all(&workspace).expect("target workspace should be created");
        let request = AgentSessionCreateRequest {
            provider: provider_name.to_string(),
            model,
            api_key,
            base_url,
            wire_api: provider::resolved_wire_api_for_provider(provider_name, None),
            workspace_path: workspace.to_string_lossy().to_string(),
            payload_root: repo_root.clone(),
            orchestrator_path: repo_root
                .join("tools")
                .join("common-he-init-orchestrator.ps1"),
        };

        let initial = store
            .create_session(request)
            .expect("live ecommerce session should start");
        assert_eq!(initial.stage, "conversation");

        let after_product = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "（1）电商网站".to_string(),
            })
            .expect("product type live prompt should succeed");
        assert!(after_product.solutions.is_empty());

        let mut after_detail = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "（2）个人消费者、企业采购，家装电器，浴霸网站\n\n（3）提升线上成交、展示品牌与产品、支持企业批量询价采购，替代线下门店导购\n\n（4）商品列表/详情、购物车下单、在线支付、企业询价、品牌展示、导购问答、后台管理、物流/售后\n\n（5）小程序、web端双端\n\n请先总结你的理解，不要直接给方案。".to_string(),
            })
            .expect("ecommerce detail live prompt should succeed");
        assert!(after_detail.solutions.is_empty());

        if !after_detail.readiness.summary_presented {
            after_detail = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: "请先总结你的理解，我确认后你再给三个方案。".to_string(),
                })
                .expect("ecommerce summary prompt should succeed");
            assert!(after_detail.solutions.is_empty());
        }

        let mut after_confirm = after_detail.clone();
        for follow_up in [
            "是的，按这个理解，请给我三个方案。",
            "总结准确，请直接给我三个方案，每个方案都包含架构、团队组成、token预估、角色选择理由和角色不选理由。",
            "我明确确认：你的理解准确。请输出三个方案。",
        ] {
            after_confirm = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: follow_up.to_string(),
                })
                .expect("ecommerce solution follow-up should succeed");

            if after_confirm.solutions.len() == 3 {
                break;
            }
        }

        assert_eq!(
            after_confirm.solutions.len(),
            3,
            "{}",
            after_confirm
                .messages
                .last()
                .map(|message| message.content.clone())
                .unwrap_or_else(|| "missing assistant reply".to_string())
        );
        assert!(after_confirm.tool_calls.iter().any(|tool_call| {
            tool_call.tool_name == "open_solution_selector" && tool_call.status == "requested"
        }));

        let chosen_solution = after_confirm
            .solutions
            .get(1)
            .or_else(|| after_confirm.solutions.first())
            .expect("three solutions should include a selectable option");
        let bootstrap_session = store
            .start_solution_bootstrap(AgentChooseRequest {
                session_id: initial.session_id.clone(),
                solution_id: chosen_solution.id.clone(),
                project_name: "浴霸电商协作包".to_string(),
                target_client: "codex".to_string(),
                selected_capabilities: vec![],
            })
            .expect("ecommerce solution bootstrap should start");
        let bootstrap_result = execute_solution_bootstrap(&bootstrap_session)
            .expect("ecommerce bootstrap should succeed after live solutions");
        assert_eq!(bootstrap_result.status, "success");
        assert!(bootstrap_result.postcheck_passed);
        assert!(!bootstrap_result.generated_files.is_empty());

        let project_context =
            fs::read_to_string(workspace.join("docs").join("project_context.md")).unwrap();
        for expected in [
            "家装电器",
            "浴霸",
            "企业询价",
            "品牌展示",
            "导购问答",
            "物流/售后",
        ] {
            assert!(project_context.contains(expected), "{expected}");
        }
        assert!(workspace.join("AGENTS.md").exists());
        assert!(workspace
            .join(".codex")
            .join("COORDINATOR-SUBAGENTS.md")
            .exists());
        let final_acceptance: Value = serde_json::from_str(
            &fs::read_to_string(
                workspace
                    .join(".commonhe")
                    .join("session")
                    .join("final-acceptance.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            final_acceptance.get("passed").and_then(Value::as_bool),
            Some(true)
        );
    }

    fn run_live_student_management_agent_flow_to_workspace(
        provider_name: &str,
        model: String,
        api_key: String,
        base_url: String,
        workspace: PathBuf,
    ) {
        let store = AgentStore::new();
        let repo_root = repo_root();
        fs::create_dir_all(&workspace).expect("target workspace should be created");
        let request = AgentSessionCreateRequest {
            provider: provider_name.to_string(),
            model,
            api_key,
            base_url,
            wire_api: provider::resolved_wire_api_for_provider(provider_name, None),
            workspace_path: workspace.to_string_lossy().to_string(),
            payload_root: repo_root.clone(),
            orchestrator_path: repo_root
                .join("tools")
                .join("common-he-init-orchestrator.ps1"),
        };

        let initial = store
            .create_session(request)
            .expect("live student management session should start");
        assert_eq!(initial.stage, "conversation");

        let after_product = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "我想做一个学生管理系统。".to_string(),
            })
            .expect("student management product type live prompt should succeed");
        assert!(after_product.solutions.is_empty());

        let mut after_detail = store
            .send_message(AgentSendRequest {
                session_id: initial.session_id.clone(),
                message: "目标用户是学校教务管理员、班主任、任课老师、学生和家长。核心目标是统一管理学生档案、班级、课程、考勤、成绩、通知和请假审批，减少 Excel 和微信群反复同步。关键功能需要学生档案、班级/课程管理、考勤记录、成绩录入与统计、通知公告、请假审批、家长/学生查询端、后台权限管理。第一期希望做 Web 管理后台和移动端查询入口。请先总结你的理解，不要直接给方案。".to_string(),
            })
            .expect("student management detail live prompt should succeed");
        assert!(after_detail.solutions.is_empty());

        if !after_detail.readiness.summary_presented {
            after_detail = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: "请先总结你的理解，我确认后你再给三个方案。".to_string(),
                })
                .expect("student management summary prompt should succeed");
            assert!(after_detail.solutions.is_empty());
        }

        let mut after_confirm = after_detail.clone();
        for follow_up in [
            "是的，按这个理解，请给我三个方案。",
            "总结准确，请直接给我三个方案，每个方案都包含架构、团队组成、token预估、角色选择理由和角色不选理由。",
            "我明确确认：你的理解准确。请输出三个方案。",
        ] {
            after_confirm = store
                .send_message(AgentSendRequest {
                    session_id: initial.session_id.clone(),
                    message: follow_up.to_string(),
                })
                .expect("student management solution follow-up should succeed");

            if after_confirm.solutions.len() == 3 {
                break;
            }
        }

        assert_eq!(
            after_confirm.solutions.len(),
            3,
            "{}",
            after_confirm
                .messages
                .last()
                .map(|message| message.content.clone())
                .unwrap_or_else(|| "missing assistant reply".to_string())
        );

        let chosen_solution = after_confirm
            .solutions
            .get(1)
            .or_else(|| after_confirm.solutions.first())
            .expect("three solutions should include a selectable option");
        let bootstrap_session = store
            .start_solution_bootstrap(AgentChooseRequest {
                session_id: initial.session_id.clone(),
                solution_id: chosen_solution.id.clone(),
                project_name: "学生管理系统协作包".to_string(),
                target_client: "codex".to_string(),
                selected_capabilities: vec![],
            })
            .expect("student management solution bootstrap should start");
        let bootstrap_result = execute_solution_bootstrap(&bootstrap_session)
            .expect("student management bootstrap should succeed after live solutions");
        assert_eq!(bootstrap_result.status, "success");
        assert!(bootstrap_result.postcheck_passed);
        assert!(!bootstrap_result.generated_files.is_empty());

        let project_context =
            fs::read_to_string(workspace.join("docs").join("project_context.md")).unwrap();
        for expected in ["学生管理", "学生档案", "班级", "考勤", "成绩", "请假"] {
            assert!(project_context.contains(expected), "{expected}");
        }
        for unexpected in ["浴霸", "家装电器", "企业询价", "导购问答", "物流/售后"]
        {
            assert!(!project_context.contains(unexpected), "{unexpected}");
        }
        assert!(workspace.join("AGENTS.md").exists());
        assert!(workspace
            .join(".codex")
            .join("COORDINATOR-SUBAGENTS.md")
            .exists());
        let final_acceptance: Value = serde_json::from_str(
            &fs::read_to_string(
                workspace
                    .join(".commonhe")
                    .join("session")
                    .join("final-acceptance.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            final_acceptance.get("passed").and_then(Value::as_bool),
            Some(true)
        );
        assert!(final_acceptance
            .get("blockingIssues")
            .and_then(Value::as_array)
            .map(|items| items.is_empty())
            .unwrap_or(true));
    }
}
