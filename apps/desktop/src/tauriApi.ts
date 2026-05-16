import { invoke } from "@tauri-apps/api/core";

function invokeArgs<T extends object>(value: T): Record<string, unknown> {
  return value as unknown as Record<string, unknown>;
}

export interface PayloadResponse {
  source: string;
  payloadRoot: string;
  orchestratorPath: string;
  checkedRoots: string[];
}

export interface ProviderModel {
  id: string;
  label: string;
}

export interface ProviderCatalogEntry {
  providerId: string;
  label: string;
  providerType: string;
  authMode: string;
  supportedModes: string[];
  discoveredModels: ProviderModel[];
  defaultModel: string;
  requiresApiKey: boolean;
  requiresBaseUrl: boolean;
  defaultBaseUrl?: string;
  supportsCustomModel: boolean;
  configDetected: boolean;
  connectivityValidated: boolean;
  blockingErrors: string[];
  userWarnings: string[];
  detectedSources: string[];
}

export interface ProviderValidationResponse {
  valid: boolean;
  normalizedProvider: string;
  authMode: string;
  requiresApiKey: boolean;
  requiresBaseUrl: boolean;
  localConfigured: boolean;
  detectedSources: string[];
  discoveredModels: ProviderModel[];
  defaultModel: string;
  defaultBaseUrl?: string;
  resolvedWireApi: string;
  providerStatus: string;
  modelStatus: string;
  authStatus: string;
  connectivityStatus: string;
  connectivityValidated: boolean;
  resolvedModel?: string;
  resolvedBaseUrl?: string;
  blockingErrors: string[];
  errors: string[];
  warnings: string[];
  userWarnings: string[];
  userFacingError?: string;
}

export interface LocalProviderStatus {
  provider: string;
  cliAvailable: boolean;
  configured: boolean;
  detectedSources: string[];
  warnings: string[];
}

export interface AgentMessage {
  role: string;
  content: string;
}

export interface AgentSolution {
  id: string;
  title: string;
  architectureSummary: string;
  teamComposition: string[];
  tokenEstimate: string;
  recommendationText: string;
  roleRationale: Record<string, string>;
  omittedRoleRationale: Record<string, string>;
}

export interface AgentToolCall {
  toolName: string;
  status: string;
  payloadJson?: string;
}

export interface AgentReadiness {
  productType?: string;
  targetUsers?: string;
  coreProblem?: string;
  keyFeatures: string[];
  constraints: string[];
  summaryPresented: boolean;
  summaryConfirmed: boolean;
  missingFields: string[];
  readyForSolutions: boolean;
}

export interface AgentBootstrapResult {
  status: "success" | "failure" | string;
  workspacePath: string;
  generatedFiles: string[];
  handoffPath?: string;
  postcheckPassed: boolean;
  userFacingMessage: string;
}

export type TargetClient = "codex" | "claude-code";

export interface SelectedCapability {
  id: string;
  label: string;
  recommended: boolean;
  selected: boolean;
  required?: boolean;
  locked?: boolean;
  status: string;
  detail: string;
}

export interface AgentSessionSnapshot {
  sessionId: string;
  stage: "conversation" | "solutions_ready" | "bootstrapping" | "bootstrap_failed" | "completed" | string;
  messages: AgentMessage[];
  understandingSummary?: string;
  readiness: AgentReadiness;
  solutions: AgentSolution[];
  toolCalls: AgentToolCall[];
  bootstrapResult?: AgentBootstrapResult;
  selectedSolutionId?: string;
  semanticReviewStatus: "not_started" | "reviewing" | "repairing" | "passed" | "failed" | string;
  semanticReviewIssues: string[];
  dialogueRoundCount: number;
  finished: boolean;
}

export interface StageResponse {
  Stage?: string;
  stage?: string;
  SessionRoot?: string;
  sessionRoot?: string;
  Message?: string;
  message?: string;
  HandoffPath?: string;
  handoffPath?: string;
  QuestionId?: string;
  questionId?: string;
  QuestionText?: string;
  questionText?: string;
  QuestionSource?: string;
  questionSource?: string;
  Choice?: string;
  choice?: string;
  Recommended?: string;
  recommended?: string;
  RecommendedOption?: string | Record<string, unknown>;
  recommendedOption?: string | Record<string, unknown>;
  Options?: Record<string, unknown>[];
  options?: Record<string, unknown>[];
  ProposalPath?: string;
  proposalPath?: string;
  Postcheck?: {
    Passed?: boolean;
    passed?: boolean;
  };
  postcheck?: {
    Passed?: boolean;
    passed?: boolean;
  };
}

export interface RunStageRequest {
  stage: string;
  sessionRoot?: string;
  projectRoot?: string;
  inputText?: string;
  choice?: string;
  targetRoot?: string;
  valuesPath?: string;
  provider?: string;
  model?: string;
  apiKey?: string;
  baseUrl?: string;
  execute?: boolean;
  force?: boolean;
}

export interface ProviderConfigRequest {
  provider: string;
  model?: string;
  apiKey?: string;
  baseUrl?: string;
}

export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

function parseJson<T>(value: unknown): T {
  if (typeof value === "string") {
    return JSON.parse(value) as T;
  }

  return value as T;
}

export async function locatePayload(): Promise<PayloadResponse> {
  if (!isTauriRuntime()) {
    return {
      source: "browser-mock",
      payloadRoot: "E:/WorkSoft/CommonHE",
      orchestratorPath: "E:/WorkSoft/CommonHE/tools/common-he-init-orchestrator.ps1",
      checkedRoots: ["E:/WorkSoft/CommonHE"],
    };
  }

  return parseJson<PayloadResponse>(await invoke("locate_payload"));
}

export async function listProviderCatalog(): Promise<ProviderCatalogEntry[]> {
  if (!isTauriRuntime()) {
    return [
      {
        providerId: "deepseek",
        label: "DeepSeek",
        providerType: "remote-api",
        authMode: "api_key",
        supportedModes: ["agent-chat"],
        discoveredModels: [
          { id: "deepseek-v4-flash", label: "deepseek-v4-flash" },
          { id: "deepseek-v4-pro", label: "deepseek-v4-pro" },
        ],
        defaultModel: "deepseek-v4-flash",
        requiresApiKey: true,
        requiresBaseUrl: false,
        defaultBaseUrl: "https://api.deepseek.com",
        supportsCustomModel: false,
        configDetected: false,
        connectivityValidated: false,
        blockingErrors: [],
        userWarnings: [],
        detectedSources: [],
      },
      {
        providerId: "codex",
        label: "Codex",
        providerType: "remote-api",
        authMode: "unsupported_official_login",
        supportedModes: ["agent-chat"],
        discoveredModels: [
          { id: "gpt-5.4", label: "gpt-5.4" },
          { id: "gpt-5", label: "gpt-5" },
        ],
        defaultModel: "gpt-5.4",
        requiresApiKey: true,
        requiresBaseUrl: false,
        defaultBaseUrl: "https://api.openai.com/v1",
        supportsCustomModel: false,
        configDetected: true,
        connectivityValidated: false,
        blockingErrors: ["codex_official_login_unsupported"],
        userWarnings: [],
        detectedSources: ["file:C:/Users/demo/.codex/auth.json", "file:C:/Users/demo/.codex/config.toml"],
      },
      {
        providerId: "custom",
        label: "其他",
        providerType: "remote-api",
        authMode: "api_key",
        supportedModes: ["agent-chat"],
        discoveredModels: [],
        defaultModel: "",
        requiresApiKey: true,
        requiresBaseUrl: true,
        supportsCustomModel: true,
        configDetected: false,
        connectivityValidated: false,
        blockingErrors: [],
        userWarnings: ["requires_remote_model_discovery"],
        detectedSources: [],
      },
    ];
  }

  return parseJson<ProviderCatalogEntry[]>(await invoke("list_provider_catalog"));
}

export async function loadProviderTools(provider: string): Promise<ProviderCatalogEntry> {
  if (!isTauriRuntime()) {
    const providers = await listProviderCatalog();
    return providers.find((entry) => entry.providerId === provider) ?? providers[0];
  }

  return parseJson<ProviderCatalogEntry>(await invoke("load_provider_tools", { provider }));
}

export async function validateProviderConfig(
  config: ProviderConfigRequest,
): Promise<ProviderValidationResponse> {
  if (!isTauriRuntime()) {
    const provider = config.provider.trim().toLowerCase();
    const officialCodex = provider === "codex" && !config.apiKey?.trim();
    const errors = [
      ...(!config.model?.trim() ? ["model_required"] : []),
      ...(officialCodex ? ["codex_official_login_unsupported"] : []),
      ...((provider === "deepseek" || provider === "custom") && !config.apiKey?.trim()
        ? ["api_key_required"]
        : []),
      ...(provider === "codex" && !officialCodex && !config.apiKey?.trim() ? ["api_key_required"] : []),
      ...(provider === "custom" && !config.baseUrl?.trim() ? ["base_url_required"] : []),
    ];
    return {
      valid: errors.length === 0,
      normalizedProvider: provider,
      authMode: officialCodex ? "unsupported_official_login" : "api_key",
      requiresApiKey: provider === "deepseek" || provider === "custom" || provider === "codex",
      requiresBaseUrl: provider === "custom",
      localConfigured: false,
      detectedSources: [],
      discoveredModels: [],
      defaultModel: config.model?.trim() ?? "",
      providerStatus: errors.length === 0 ? "ready_for_connectivity_check" : "blocked",
      modelStatus: config.model?.trim() ? "selected" : "missing",
      authStatus: officialCodex ? "official_login" : config.apiKey?.trim() ? "resolved" : "missing",
      connectivityStatus: "not_checked",
      connectivityValidated: false,
      resolvedModel: config.model?.trim(),
      resolvedBaseUrl: config.baseUrl?.trim(),
      resolvedWireApi: provider === "codex" ? "responses" : "chat_completions",
      blockingErrors: errors,
      errors,
      warnings: ["browser_mock"],
      userWarnings: ["browser_mock"],
      userFacingError:
        errors[0] === "api_key_required"
          ? "请填写APIKey。"
          : errors[0] === "codex_official_login_unsupported"
            ? "当前不支持官方 Codex 登录授权。请选择 Codex 的 OpenAI Responses API 协议配置，并提供 APIKey。"
          : errors[0] === "base_url_required"
            ? "请填写 API Base URL。"
            : errors[0] === "model_required"
              ? "请先选择模型，或显式切换到自定义模型。"
              : undefined,
    };
  }

  return parseJson<ProviderValidationResponse>(
    await invoke("validate_provider_config", invokeArgs(config)),
  );
}

export async function validateProviderConnection(
  config: ProviderConfigRequest,
): Promise<ProviderValidationResponse> {
  if (!isTauriRuntime()) {
    return {
      ...(await validateProviderConfig(config)),
      connectivityStatus: "validated",
      connectivityValidated: true,
      providerStatus: "validated",
    };
  }

  return parseJson<ProviderValidationResponse>(
    await invoke("validate_provider_connection", invokeArgs(config)),
  );
}

export async function scanLocalProviders(): Promise<LocalProviderStatus[]> {
  if (!isTauriRuntime()) {
    return [
      {
        provider: "codex",
        cliAvailable: false,
        configured: false,
        detectedSources: [],
        warnings: ["browser_mock"],
      },
      {
        provider: "claude-code",
        cliAvailable: false,
        configured: false,
        detectedSources: [],
        warnings: ["browser_mock"],
      },
      {
        provider: "gemini-cli",
        cliAvailable: false,
        configured: false,
        detectedSources: [],
        warnings: ["browser_mock"],
      },
    ];
  }

  return parseJson<LocalProviderStatus[]>(await invoke("scan_local_providers"));
}

export async function discoverProviderModels(request: {
  provider: string;
  apiKey?: string;
  baseUrl?: string;
}): Promise<ProviderModel[]> {
  if (!isTauriRuntime()) {
    if (request.provider === "custom") {
      return [
        { id: "gpt-5.4", label: "gpt-5.4" },
        { id: "deepseek-v4-flash", label: "deepseek-v4-flash" },
      ];
    }
    return [];
  }

  return parseJson<ProviderModel[]>(await invoke("discover_provider_models", invokeArgs(request)));
}

export async function createAgentSession(request: {
  provider: string;
  model: string;
  apiKey: string;
  baseUrl: string;
  wireApi: string;
  workspacePath: string;
}): Promise<AgentSessionSnapshot> {
  if (!isTauriRuntime()) {
    return {
      sessionId: "mock-session",
      stage: "conversation",
      messages: [
        {
          role: "assistant",
          content:
            "我是梦星星。先告诉我，你想做的是 MCP、Skill、网站、软件，还是其他产品？",
        },
      ],
      readiness: {
        keyFeatures: [],
        constraints: [],
        summaryPresented: false,
        summaryConfirmed: false,
        missingFields: ["产品形态", "目标用户", "核心问题", "关键功能", "约束条件", "用户确认"],
        readyForSolutions: false,
      },
      solutions: [],
      toolCalls: [],
      semanticReviewStatus: "not_started",
      semanticReviewIssues: [],
      dialogueRoundCount: 0,
      finished: false,
    };
  }

  return parseJson<AgentSessionSnapshot>(await invoke("create_agent_session", invokeArgs(request)));
}

export async function sendAgentMessage(request: {
  sessionId: string;
  message: string;
}): Promise<AgentSessionSnapshot> {
  if (!isTauriRuntime()) {
    return {
      sessionId: request.sessionId,
      stage: "conversation",
      messages: [
        { role: "user", content: request.message },
        {
          role: "assistant",
          content: "请继续补充目标用户、核心功能和你更偏好的实现形态。",
        },
      ],
      readiness: {
        keyFeatures: [],
        constraints: [],
        summaryPresented: false,
        summaryConfirmed: false,
        missingFields: ["目标用户", "核心问题", "关键功能", "约束条件", "用户确认"],
        readyForSolutions: false,
      },
      solutions: [],
      toolCalls: [],
      semanticReviewStatus: "not_started",
      semanticReviewIssues: [],
      dialogueRoundCount: 0,
      finished: false,
    };
  }

  return parseJson<AgentSessionSnapshot>(await invoke("send_agent_message", invokeArgs(request)));
}

export async function chooseAgentSolution(request: {
  sessionId: string;
  solutionId: string;
  projectName: string;
  targetClient: TargetClient;
  selectedCapabilities: SelectedCapability[];
}): Promise<AgentSessionSnapshot> {
  if (!isTauriRuntime()) {
    return {
      sessionId: request.sessionId,
      stage: "completed",
      messages: [
        {
          role: "assistant",
          content: "星梦梦语义验收通过，初始化流程结束。请在新会话中继续后续实施。",
        },
      ],
      readiness: {
        productType: "网站",
        targetUsers: "测试用户",
        coreProblem: "验证初始化流程",
        keyFeatures: ["对话", "方案选择", "初始化落盘"],
        constraints: ["本地演示"],
        summaryPresented: true,
        summaryConfirmed: true,
        missingFields: [],
        readyForSolutions: true,
      },
      solutions: [],
      toolCalls: [
        {
          toolName: "open_solution_selector",
          status: "completed",
          payloadJson: JSON.stringify({ selectedSolutionId: request.solutionId }),
        },
      ],
      bootstrapResult: {
        status: "success",
        workspacePath: "E:/Mock/Workspace",
        generatedFiles: ["AGENTS.md", "docs/project_context.md"],
        handoffPath: "E:/Mock/Workspace/.commonhe/session/bootstrap-handoff.md",
        postcheckPassed: true,
        userFacingMessage: "初始化成功，星梦梦语义验收与 postcheck 均已通过。请在新会话中继续实施。",
      },
      selectedSolutionId: request.solutionId,
      semanticReviewStatus: "passed",
      semanticReviewIssues: [],
      dialogueRoundCount: 1,
      finished: true,
    };
  }

  return parseJson<AgentSessionSnapshot>(await invoke("choose_agent_solution", invokeArgs(request)));
}

export async function runOrchestratorStage(request: RunStageRequest): Promise<StageResponse> {
  if (!isTauriRuntime()) {
    return {
      Stage: request.stage === "start" ? "implementation_ready" : request.stage,
      Message:
        request.stage === "start"
          ? "Mock initialization completed with postcheck passed."
          : "Mock stage completed.",
      HandoffPath: `${request.projectRoot ?? "E:/Projects/Demo"}/.commonhe/session/bootstrap-handoff.md`,
      Postcheck: { Passed: true },
    };
  }

  return parseJson<StageResponse>(await invoke("run_orchestrator_stage", invokeArgs(request)));
}

export async function readStatus(projectRoot?: string, sessionRoot?: string): Promise<unknown> {
  if (!isTauriRuntime()) {
    return { exists: true, status: { stage: "implementation_ready" } };
  }

  return parseJson(await invoke("read_status", { projectRoot, sessionRoot }));
}

export async function openExternalUrl(url: string): Promise<void> {
  if (!isTauriRuntime()) {
    window.open(url, "_blank", "noopener,noreferrer");
    return;
  }

  await invoke("open_external_url", { url });
}

export async function pickWorkspaceDirectory(): Promise<string | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({
    directory: true,
    multiple: false,
    title: "选择星星的vibecoding启动器工作空间",
  });

  return typeof selected === "string" ? selected : null;
}
