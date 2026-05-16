import { buildAgentStartLog } from "./agentFlow.ts";

export const FINAL_SUCCESS_TEXT = "星星，加班成功了~这就睡觉去了哦";

export type WorkflowStepId = "scan" | "provider" | "workspace" | "initialize" | "handoff";

export type CapabilityStatus = "pass" | "warn" | "fail" | "unknown";

export interface WorkflowStep {
  id: WorkflowStepId;
  label: string;
}

export interface CapabilityResult {
  name: string;
  status: CapabilityStatus;
  detail: string;
}

export const MANDATORY_WORKFLOW_CAPABILITY_IDS = [
  "superpowers",
  "agent-browser",
  "chrome-devtools",
  "GitNexus",
  "Speckit",
] as const;

export interface MandatorySelectedCapability {
  id: string;
  label: string;
  recommended: boolean;
  selected: true;
  required: true;
  locked: true;
  status: string;
  detail: string;
}

const MANDATORY_WORKFLOW_CAPABILITY_DETAILS: Record<
  (typeof MANDATORY_WORKFLOW_CAPABILITY_IDS)[number],
  { label: string; detail: string }
> = {
  superpowers: {
    label: "superpowers / required skills",
    detail:
      "skill_presence: using-superpowers, test-driven-development；使用包内预装或团队发布方式安装 using-superpowers 与 test-driven-development",
  },
  "agent-browser": {
    label: "agent-browser",
    detail: "~/.agent-browser/config.json or ~/.roxybrowser/shortcuts.json；使用包内说明或官方仓库安装 agent-browser",
  },
  "chrome-devtools": {
    label: "chrome-devtools MCP",
    detail: "~/.codex/config.toml or .mcp.json contains chrome-devtools；codex mcp add chrome-devtools -- npx chrome-devtools-mcp@latest",
  },
  GitNexus: {
    label: "GitNexus",
    detail: "gitnexus --version；npm install -g gitnexus",
  },
  Speckit: {
    label: "Speckit",
    detail: "specify --version；uv tool install specify-cli",
  },
};

export function buildMandatorySelectedCapabilities(_targetClient: string): MandatorySelectedCapability[] {
  return ["superpowers", "chrome-devtools", "GitNexus", "Speckit", "agent-browser"].map((id) => {
    const capabilityId = id as (typeof MANDATORY_WORKFLOW_CAPABILITY_IDS)[number];
    const capability = MANDATORY_WORKFLOW_CAPABILITY_DETAILS[capabilityId];
    return {
      id: capabilityId,
      label: capability.label,
      recommended: true,
      selected: true,
      required: true,
      locked: true,
      status: "pending",
      detail: capability.detail,
    };
  });
}

export interface ProviderSelection {
  providerId: string;
  providerLabel: string;
  model?: string;
}

export interface ProviderToolStatus {
  provider: string;
  cliAvailable: boolean;
  configured: boolean;
  detectedSources: string[];
  warnings: string[];
}

export interface WorkspaceSelection {
  path: string;
  kind: "existing" | "empty" | "unknown";
  signal: string;
}

export interface InitializationResult {
  status: "success" | "failure";
  stage: string;
  message: string;
  handoffPath?: string;
  logs: string[];
}

export interface OrchestratorStageResult {
  Stage?: unknown;
  stage?: unknown;
  Message?: unknown;
  message?: unknown;
  HandoffPath?: unknown;
  handoffPath?: unknown;
  QuestionId?: unknown;
  questionId?: unknown;
  QuestionText?: unknown;
  questionText?: unknown;
  QuestionSource?: unknown;
  questionSource?: unknown;
  Recommended?: unknown;
  recommended?: unknown;
  RecommendedOption?: unknown;
  recommendedOption?: unknown;
  Options?: unknown;
  options?: unknown;
  ProposalPath?: unknown;
  proposalPath?: unknown;
  Postcheck?: {
    Passed?: unknown;
    passed?: unknown;
  };
  postcheck?: {
    Passed?: unknown;
    passed?: unknown;
  };
}

export interface AppState {
  step: WorkflowStepId;
  payloadPath?: string;
  capabilities: CapabilityResult[];
  provider?: ProviderSelection;
  providerToolStatus?: ProviderToolStatus;
  workspace?: WorkspaceSelection;
  outcome?: "success" | "failure";
  result?: InitializationResult;
  errorMessage?: string;
  recoverableError?: string;
  logs: string[];
  historyLogs: string[];
  diagnosticLogs: string[];
}

export interface DiagnosticLogPayload {
  operation: string;
  severity?: "info" | "warn" | "error";
  timestamp?: string;
  provider?: string;
  model?: string;
  baseUrl?: string;
  wireApi?: string;
  detail: string;
}

export type AppAction =
  | { type: "scan.complete"; payload: { payloadPath: string; capabilities: CapabilityResult[] } }
  | { type: "provider.tools.loaded"; payload: ProviderToolStatus }
  | { type: "provider.validated"; payload: ProviderSelection }
  | { type: "provider.reselect_requested" }
  | { type: "workspace.selected"; payload: WorkspaceSelection }
  | { type: "initialize.started" }
  | { type: "initialize.succeeded"; payload: InitializationResult }
  | { type: "initialize.failed"; payload: unknown }
  | { type: "initialize.log"; payload: string }
  | { type: "initialize.diagnostic"; payload: DiagnosticLogPayload }
  | { type: "initialize.recoverableError"; payload?: string }
  | { type: "reset" };

export const WORKFLOW_STEPS: WorkflowStep[] = [
  { id: "scan", label: "环境" },
  { id: "provider", label: "渠道" },
  { id: "workspace", label: "工作区" },
  { id: "initialize", label: "对话" },
  { id: "handoff", label: "收口" },
];

export function createInitialState(): AppState {
  return {
    step: "scan",
    capabilities: [],
    logs: [],
    historyLogs: [],
    diagnosticLogs: [],
    recoverableError: undefined,
  };
}

export function reducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "scan.complete":
      return {
        ...state,
        step: "provider",
        payloadPath: action.payload.payloadPath,
        capabilities: action.payload.capabilities,
        logs: [...state.logs, "本地环境检测已完成。"],
      };
    case "provider.tools.loaded":
      return {
        ...state,
        providerToolStatus: action.payload,
        logs: [...state.logs, `渠道工具已加载：${action.payload.provider}`],
      };
    case "provider.validated":
      return {
        ...state,
        step: "workspace",
        provider: action.payload,
        logs: [...state.logs, `渠道已验证：${action.payload.providerLabel}`],
      };
    case "provider.reselect_requested":
      return {
        ...state,
        step: "provider",
        provider: undefined,
        workspace: undefined,
        outcome: undefined,
        result: undefined,
        errorMessage: undefined,
        recoverableError: undefined,
        logs: [...state.logs, "已返回渠道与模型设置，请重新验证渠道和模型后再继续。"],
      };
    case "workspace.selected":
      return {
        ...state,
        step: "initialize",
        workspace: action.payload,
        outcome: undefined,
        result: undefined,
        errorMessage: undefined,
        recoverableError: undefined,
        historyLogs: state.logs.length > 0 ? [...state.historyLogs, ...state.logs] : state.historyLogs,
        logs: [`工作区已锁定：${action.payload.path}`],
        diagnosticLogs: [],
      };
    case "initialize.started":
      return {
        ...state,
        step: "initialize",
        outcome: undefined,
        result: undefined,
        errorMessage: undefined,
        recoverableError: undefined,
        historyLogs: state.logs.length > 0 ? [...state.historyLogs, ...state.logs] : state.historyLogs,
        logs: [buildAgentStartLog()],
        diagnosticLogs: [],
      };
    case "initialize.succeeded":
      return initializeSucceeded(state, action.payload);
    case "initialize.failed":
      return initializeFailed(state, action.payload);
    case "initialize.log":
      if (state.logs[state.logs.length - 1] === action.payload) {
        return state;
      }
      return {
        ...state,
        logs: [...state.logs, action.payload],
      };
    case "initialize.diagnostic": {
      const diagnostic = formatDiagnosticLog(action.payload);
      if (state.diagnosticLogs[state.diagnosticLogs.length - 1] === diagnostic) {
        return state;
      }
      return {
        ...state,
        diagnosticLogs: [...state.diagnosticLogs, diagnostic],
      };
    }
    case "initialize.recoverableError":
      return {
        ...state,
        recoverableError: action.payload,
      };
    case "reset":
      return createInitialState();
    default:
      return state;
  }
}

export function formatRecoverableRuntimeErrorForUser(
  message: string,
  operation: "session" | "message" | "bootstrap" = "message",
): string {
  if (message.includes("模型请求失败") || message.includes("error sending request for url")) {
    if (operation === "message") {
      return "梦星星这轮回复没有生成成功：模型服务暂时连不上，本轮请求已经结束，窗口已恢复。你可以直接重试，或先切换渠道、模型、网络代理/API Base URL。";
    }
    if (operation === "bootstrap") {
      return "初始化收口没有继续执行：模型服务暂时连不上，本轮请求已经结束，窗口已恢复到能力选择页。请检查渠道、模型、网络代理/API Base URL 后重试。";
    }
    return "模型服务暂时连不上，本轮请求已经结束，窗口已恢复。请先检查当前渠道的网络、代理、防火墙或 API Base URL，然后可以直接重试。";
  }
  if (message === "agent_auth_failed" || message.includes("认证失败")) {
    return "模型渠道认证失败，本轮请求已经结束。请检查 APIKey 或本地登录状态后重试。";
  }
  if (message.includes("codex_official_login_unsupported")) {
    return "当前不支持官方 Codex 登录授权。本轮请求已经结束；请返回渠道设置，改用 Codex 的 OpenAI Responses API 配置并提供 APIKey。";
  }
  if (message.includes("agent_response_invalid") || message.includes("semantic_agent_response_invalid")) {
    return "模型返回内容格式不符合要求，本轮请求已经结束。你可以直接重试；如果连续出现，请切换模型或渠道。";
  }
  return `${message}。本轮请求已结束，窗口已恢复；请按提示处理后重试。`;
}

export function formatDiagnosticLog(payload: DiagnosticLogPayload): string {
  const severity = payload.severity ?? "info";
  const timestamp = payload.timestamp ?? new Date().toISOString();
  const parts = [
    `[diagnostic:${severity}]`,
    `time=${timestamp}`,
    `operation=${payload.operation || "unknown"}`,
    payload.provider ? `provider=${payload.provider}` : undefined,
    payload.model ? `model=${payload.model}` : undefined,
    payload.baseUrl ? `baseUrl=${payload.baseUrl}` : undefined,
    payload.wireApi ? `wireApi=${payload.wireApi}` : undefined,
    `detail=${sanitizeDiagnosticDetail(payload.detail)}`,
  ].filter(Boolean);
  return parts.join(" ");
}

export function sanitizeDiagnosticDetail(detail: string): string {
  return detail
    .replace(/(api[_-]?key=)[^&\s]+/gi, "$1[redacted]")
    .replace(/(authorization:\s*bearer\s+)[^\s]+/gi, "$1[redacted]")
    .replace(/(sk-[a-z0-9_-]{8,})/gi, "[redacted-api-key]");
}

export function initializeSucceeded(state: AppState, result: InitializationResult): AppState {
  return {
    ...state,
    step: "handoff",
    outcome: "success",
    result,
    logs: [...state.logs, ...result.logs],
  };
}

export function initializeFailed(state: AppState, error: unknown): AppState {
  const errorMessage = error instanceof Error ? error.message : String(error);
  return {
    ...state,
    step: "handoff",
    outcome: "failure",
    errorMessage,
    result: {
      status: "failure",
      stage: "error",
      message: errorMessage,
      logs: [errorMessage],
    },
    logs: [...state.logs, errorMessage],
  };
}

function readString(value: unknown): string | undefined {
  return typeof value === "string" && value.trim() ? value.trim() : undefined;
}

function readStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) {
    return [];
  }

  return value
    .map((item) => readString(item))
    .filter((item): item is string => Boolean(item));
}

function readRecord(value: unknown): Record<string, unknown> | undefined {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : undefined;
}

function readRecordString(record: Record<string, unknown> | undefined, key: string): string | undefined {
  return record ? readString(record[key]) : undefined;
}

export function getInitializationStage(result: OrchestratorStageResult): string {
  return readString(result.Stage) ?? readString(result.stage) ?? "unknown";
}

function getInitializationMessage(result: OrchestratorStageResult): string | undefined {
  return readString(result.Message) ?? readString(result.message);
}

function getInitializationHandoffPath(result: OrchestratorStageResult): string | undefined {
  return readString(result.HandoffPath) ?? readString(result.handoffPath);
}

function getQuestionId(result: OrchestratorStageResult): string | undefined {
  return readString(result.QuestionId) ?? readString(result.questionId);
}

function getQuestionText(result: OrchestratorStageResult): string | undefined {
  return readString(result.QuestionText) ?? readString(result.questionText);
}

function hasPassedPostcheck(result: OrchestratorStageResult): boolean {
  const postcheck = result.Postcheck ?? result.postcheck;
  return postcheck?.Passed === true || postcheck?.passed === true;
}

export function isFinalInitializationResult(result: OrchestratorStageResult): boolean {
  const stage = getInitializationStage(result);
  return (
    stage === "implementation_ready" ||
    stage === "completed_closure" ||
    (stage === "postcheck" && hasPassedPostcheck(result))
  );
}

export function isContinuableInitializationResult(result: OrchestratorStageResult): boolean {
  return ["discovery", "proposal_ready", "proposal", "confirmed", "bootstrap_preview"].includes(
    getInitializationStage(result),
  );
}

export function buildInitializationBlockError(result: OrchestratorStageResult): Error {
  const stage = getInitializationStage(result);
  const message = getInitializationMessage(result);
  const questionId = getQuestionId(result);
  const questionText = getQuestionText(result);
  const details = [message, questionId ? `QuestionId: ${questionId}` : undefined, questionText]
    .filter(Boolean)
    .join(" ");

  return new Error(
    `初始化尚未完成：当前处于 ${stage}。${details || "orchestrator 未返回最终 postcheck 成功状态。"}`,
  );
}

export function buildInitializationSuccessResult(
  result: OrchestratorStageResult,
): InitializationResult {
  if (!isFinalInitializationResult(result)) {
    throw buildInitializationBlockError(result);
  }

  return {
    status: "success",
    stage: getInitializationStage(result),
    message: getInitializationMessage(result) ?? FINAL_SUCCESS_TEXT,
    handoffPath: getInitializationHandoffPath(result),
    logs: ["doctor passed", "precheck passed", "postcheck passed"],
  };
}

export interface ProposalDisplayOption {
  id: string;
  name: string;
  positioning?: string;
  timeCost?: string;
  deploymentCost?: string;
  developmentDifficulty?: string;
  scalability?: string;
  risks?: string;
  enabledRoles: string[];
  recommendationReason?: string;
  domainModules: string[];
}

export interface ProposalDisplay {
  recommendedId?: string;
  recommendedOption?: ProposalDisplayOption;
  options: ProposalDisplayOption[];
  proposalPath?: string;
}

function normalizeProposalOption(value: unknown): ProposalDisplayOption | undefined {
  const record = readRecord(value);
  if (!record) {
    return undefined;
  }

  const id = readRecordString(record, "id");
  const name = readRecordString(record, "name");
  if (!id || !name) {
    return undefined;
  }

  return {
    id,
    name,
    positioning: readRecordString(record, "positioning"),
    timeCost: readRecordString(record, "time_cost") ?? readRecordString(record, "timeCost"),
    deploymentCost: readRecordString(record, "deployment_cost") ?? readRecordString(record, "deploymentCost"),
    developmentDifficulty:
      readRecordString(record, "development_difficulty") ?? readRecordString(record, "developmentDifficulty"),
    scalability: readRecordString(record, "scalability"),
    risks: readRecordString(record, "risks"),
    enabledRoles: readStringArray(record.enabled_roles ?? record.enabledRoles),
    recommendationReason:
      readRecordString(record, "recommendation_reason") ?? readRecordString(record, "recommendationReason"),
    domainModules: readStringArray(record.domain_modules ?? record.domainModules),
  };
}

export function extractProposalDisplay(result: OrchestratorStageResult): ProposalDisplay {
  const rawOptions = Array.isArray(result.Options)
    ? result.Options
    : Array.isArray(result.options)
      ? result.options
      : [];
  const options = rawOptions
    .map((option) => normalizeProposalOption(option))
    .filter((option): option is ProposalDisplayOption => Boolean(option));
  const recommendedId =
    readString(result.Recommended) ??
    readString(result.recommended) ??
    normalizeProposalOption(result.RecommendedOption ?? result.recommendedOption)?.id ??
    readString(result.RecommendedOption ?? result.recommendedOption);
  const normalizedRecommendedOption = normalizeProposalOption(result.RecommendedOption ?? result.recommendedOption);
  const mergedOptions =
    normalizedRecommendedOption && options.some((option) => option.id === normalizedRecommendedOption.id)
      ? options.map((option) =>
          option.id === normalizedRecommendedOption.id ? { ...option, ...normalizedRecommendedOption } : option,
        )
      : options;
  const recommendedOption =
    mergedOptions.find((option) => option.id === recommendedId) ?? normalizedRecommendedOption;

  return {
    recommendedId,
    recommendedOption,
    options: mergedOptions,
    proposalPath: readString(result.ProposalPath) ?? readString(result.proposalPath),
  };
}

export interface InitializationStageRequestLike {
  stage: string;
  sessionRoot?: string;
  projectRoot?: string;
  inputText?: string;
  choice?: string;
  targetRoot?: string;
  execute?: boolean;
  projectName?: string;
  targetClient?: string;
  selectedCapabilities?: string[];
}

export function validateInitializationStageRequest(request: InitializationStageRequestLike): void {
  const stage = request.stage;
  if ((stage === "start" || stage === "doctor" || stage === "precheck") && !request.projectRoot?.trim()) {
    throw new Error(`projectRoot is required for ${stage} stage.`);
  }
  if (["answer", "propose", "confirm", "bootstrap"].includes(stage) && !request.sessionRoot?.trim()) {
    throw new Error(`sessionRoot is required for ${stage} stage.`);
  }
  if (stage === "answer" && !request.inputText?.trim()) {
    throw new Error("inputText is required for answer stage.");
  }
  if (stage === "confirm" && !request.choice?.trim()) {
    throw new Error("choice is required for confirm stage.");
  }
  if (stage === "bootstrap" && !request.targetRoot?.trim()) {
    throw new Error("targetRoot is required for bootstrap stage.");
  }
  if (stage === "bootstrap" && !request.projectName?.trim()) {
    throw new Error("projectName is required for bootstrap stage.");
  }
  if (
    stage === "bootstrap" &&
    request.targetClient !== "codex" &&
    request.targetClient !== "claude-code"
  ) {
    throw new Error("targetClient must be codex or claude-code for bootstrap stage.");
  }
}
