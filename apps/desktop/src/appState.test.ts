import assert from "node:assert/strict";
import {
  FINAL_SUCCESS_TEXT,
  MANDATORY_WORKFLOW_CAPABILITY_IDS,
  WORKFLOW_STEPS,
  buildInitializationBlockError,
  buildInitializationSuccessResult,
  buildMandatorySelectedCapabilities,
  createInitialState,
  extractProposalDisplay,
  formatRecoverableRuntimeErrorForUser,
  initializeFailed,
  initializeSucceeded,
  isContinuableInitializationResult,
  isFinalInitializationResult,
  reducer,
  validateInitializationStageRequest,
} from "./appState.ts";
import {
  buildAgentStartLog,
  createBootstrapProgressItems,
  getBootstrapBusyNotice,
  getConversationStatusView,
  getBootstrapButtonLabel,
  getBootstrapProgressIcon,
  getConversationTitle,
  getThinkingMascotAriaLabel,
  getSendButtonLabel,
  getSendButtonViewLabel,
  getProviderAvailabilityText,
  getTargetClientOptions,
  getWorkspacePathChipText,
  shouldDisableProviderCard,
  shouldRenderProviderCard,
  shouldShowSolutionDialog,
  shouldRenderDiagnosticLogs,
  CUSTOMER_SERVICE_CONTACT,
  CUSTOMER_SERVICE_COPIED_TOOLTIP,
  CUSTOMER_SERVICE_TOOLTIP,
  GITHUB_PROFILE_TOOLTIP,
  GITHUB_PROFILE_URL,
  WORK_IN_PROGRESS_TEXT,
  STAR_NAME,
} from "./agentFlow.ts";

const state = createInitialState();

assert.equal(WORKFLOW_STEPS.length, 5);
assert.deepEqual(
  WORKFLOW_STEPS.map((step) => step.id),
  ["scan", "provider", "workspace", "initialize", "handoff"],
);
assert.deepEqual(
  WORKFLOW_STEPS.map((step) => step.label),
  ["环境", "渠道", "工作区", "对话", "收口"],
);
assert.equal(FINAL_SUCCESS_TEXT, "星星，加班成功了~这就睡觉去了哦");
assert.equal(state.step, "scan");
assert.deepEqual(MANDATORY_WORKFLOW_CAPABILITY_IDS, [
  "superpowers",
  "agent-browser",
  "chrome-devtools",
  "GitNexus",
  "Speckit",
]);
assert.deepEqual(
  buildMandatorySelectedCapabilities("codex").map((capability) => ({
    id: capability.id,
    selected: capability.selected,
    required: capability.required,
    locked: capability.locked,
  })),
  [
    { id: "superpowers", selected: true, required: true, locked: true },
    { id: "chrome-devtools", selected: true, required: true, locked: true },
    { id: "GitNexus", selected: true, required: true, locked: true },
    { id: "Speckit", selected: true, required: true, locked: true },
    { id: "agent-browser", selected: true, required: true, locked: true },
  ],
);

const scanned = reducer(state, {
  type: "scan.complete",
  payload: {
    payloadPath: "E:/WorkSoft/CommonHE",
    capabilities: [
      { name: "superpowers", status: "pass", detail: "loaded" },
      { name: "agent-browser", status: "pass", detail: "available" },
    ],
  },
});

assert.equal(scanned.step, "provider");
assert.equal(scanned.payloadPath, "E:/WorkSoft/CommonHE");
assert.equal(scanned.capabilities.length, 2);

const toolsLoaded = reducer(scanned, {
  type: "provider.tools.loaded",
  payload: {
    provider: "codex",
    cliAvailable: true,
    configured: true,
    detectedSources: ["file:C:/Users/Star/.codex/auth.json"],
    warnings: [],
  },
});

assert.equal(toolsLoaded.providerToolStatus?.provider, "codex");
assert.equal(toolsLoaded.providerToolStatus?.configured, true);
assert.ok(toolsLoaded.logs.includes("渠道工具已加载：codex"));

const provider = reducer(scanned, {
  type: "provider.validated",
  payload: {
    providerId: "codex",
    providerLabel: "Codex",
    model: "gpt-5",
  },
});

assert.equal(provider.step, "workspace");
assert.equal(provider.provider?.providerId, "codex");
assert.ok(provider.logs.includes("渠道已验证：Codex"));

const workspace = reducer(provider, {
  type: "workspace.selected",
  payload: {
    path: "E:/Projects/Product",
    kind: "existing",
    signal: "Detected package.json and git history",
  },
});

assert.equal(workspace.step, "initialize");
assert.equal(workspace.workspace?.kind, "existing");
assert.deepEqual(workspace.logs, ["工作区已锁定：E:/Projects/Product"]);

const runStarted = reducer(
  {
    ...workspace,
    logs: ["old scan error"],
    historyLogs: [],
  },
  { type: "initialize.started" },
);

assert.equal(runStarted.step, "initialize");
assert.deepEqual(
  runStarted.logs,
  ["梦星星会话已启动，接下来会通过自然语言对话澄清需求并准备三套方案。"],
);
assert.ok(runStarted.historyLogs.includes("old scan error"));

const dedupedLogState = reducer(
  reducer(runStarted, {
    type: "initialize.log",
    payload: "模型请求失败：无法连接到模型服务，请检查网络、代理、防火墙和 API Base URL。",
  }),
  {
    type: "initialize.log",
    payload: "模型请求失败：无法连接到模型服务，请检查网络、代理、防火墙和 API Base URL。",
  },
);

assert.equal(
  dedupedLogState.logs.filter((log) => log.includes("模型请求失败")).length,
  1,
);

const diagnosticState = reducer(runStarted, {
  type: "initialize.diagnostic",
  payload: {
    operation: "message",
    severity: "error",
    timestamp: "2026-05-12T08:00:00.000Z",
    provider: "codex",
    model: "gpt-5.4",
    baseUrl: "https://ai.xingmengmeng.com/v1",
    wireApi: "responses",
    detail:
      "error sending request for url (https://ai.xingmengmeng.com/v1/chat/completions) api_key=test-api-key-secret123456",
  },
});

assert.equal(diagnosticState.diagnosticLogs.length, 1);
assert.match(diagnosticState.diagnosticLogs[0], /\[diagnostic:error\]/);
assert.match(diagnosticState.diagnosticLogs[0], /time=2026-05-12T08:00:00\.000Z/);
assert.match(diagnosticState.diagnosticLogs[0], /operation=message/);
assert.match(diagnosticState.diagnosticLogs[0], /provider=codex/);
assert.match(diagnosticState.diagnosticLogs[0], /model=gpt-5\.4/);
assert.match(diagnosticState.diagnosticLogs[0], /baseUrl=https:\/\/ai\.xingmengmeng\.com\/v1/);
assert.match(diagnosticState.diagnosticLogs[0], /wireApi=responses/);
assert.match(diagnosticState.diagnosticLogs[0], /chat\/completions/);
assert.doesNotMatch(diagnosticState.diagnosticLogs[0], /test-api-key-secret123456/);
assert.match(diagnosticState.diagnosticLogs[0], /api_key=\[redacted\]/);

const detailedDiagnosticState = reducer(runStarted, {
  type: "initialize.diagnostic",
  payload: {
    operation: "message",
    severity: "error",
    timestamp: "2026-05-14T07:22:23.106Z",
    provider: "deepseek",
    model: "deepseek-v4-flash",
    baseUrl: "https://api.deepseek.com",
    wireApi: "chat_completions",
    detail:
      "agent_response_invalid status=200 endpoint=https://api.deepseek.com/chat/completions bodySnippet=data: {\"error\":\"bad\"} api_key=live-test-api-key-secret123456",
  },
});

assert.match(detailedDiagnosticState.diagnosticLogs[0], /status=200/);
assert.match(detailedDiagnosticState.diagnosticLogs[0], /endpoint=https:\/\/api\.deepseek\.com\/chat\/completions/);
assert.match(detailedDiagnosticState.diagnosticLogs[0], /bodySnippet=data:/);
assert.doesNotMatch(detailedDiagnosticState.diagnosticLogs[0], /live-test-api-key-secret123456/);

assert.equal(
  formatRecoverableRuntimeErrorForUser(
    "模型请求失败：无法连接到模型服务，请检查网络、代理、防火墙和 API Base URL。底层错误：error sending request for url (https://ai.xingmengmeng.com/v1/chat/completions)",
  ),
  "梦星星这轮回复没有生成成功：模型服务暂时连不上，本轮请求已经结束，窗口已恢复。你可以直接重试，或先切换渠道、模型、网络代理/API Base URL。",
);

assert.equal(
  formatRecoverableRuntimeErrorForUser("codex_official_login_unsupported", "session"),
  "当前不支持官方 Codex 登录授权。本轮请求已经结束；请返回渠道设置，改用 Codex 的 OpenAI Responses API 配置并提供 APIKey。",
);

const recoverableErrorState = reducer(runStarted, {
  type: "initialize.recoverableError",
  payload: "梦星星这轮回复没有生成成功",
});
assert.equal(recoverableErrorState.recoverableError, "梦星星这轮回复没有生成成功");

const providerRecoveryState = reducer(
  {
    ...recoverableErrorState,
    provider: {
      providerId: "codex",
      providerLabel: "Codex",
      model: "gpt-5.5",
    },
    workspace: {
      path: "E:/test/test-shop",
      kind: "existing",
      signal: "locked",
    },
  },
  { type: "provider.reselect_requested" },
);
assert.equal(providerRecoveryState.step, "provider");
assert.equal(providerRecoveryState.recoverableError, undefined);
assert.equal(providerRecoveryState.provider, undefined);
assert.equal(providerRecoveryState.workspace, undefined);
assert.ok(providerRecoveryState.logs[providerRecoveryState.logs.length - 1]?.includes("重新验证渠道和模型"));

assert.equal(
  buildAgentStartLog(),
  "梦星星会话已启动，接下来会通过自然语言对话澄清需求并准备三套方案。",
);
assert.equal(STAR_NAME, "梦星星");
assert.equal(getConversationTitle(), "和梦星星对话");
assert.equal(getSendButtonLabel(false), "发送给梦星星");
assert.equal(getSendButtonLabel(true), "梦星星思考中...");
assert.deepEqual(
  getConversationStatusView({
    busyNotice: null,
    recoverableError: "梦星星这轮回复没有生成成功",
  }),
  {
    kind: "error",
    message: "梦星星这轮回复没有生成成功",
  },
);
assert.equal(
  getSendButtonViewLabel({
    conversationBusy: false,
    recoverableError: "梦星星这轮回复没有生成成功",
  }),
  "重试发送给梦星星",
);
assert.notEqual(
  getSendButtonViewLabel({
    conversationBusy: false,
    recoverableError: "梦星星这轮回复没有生成成功",
  }),
  "梦星星思考中...",
);
assert.equal(getBootstrapButtonLabel(false), "确认方案并开始初始化");
assert.equal(getBootstrapButtonLabel(true), "星梦梦正在复核，梦星星正在整理初始化结果...");
assert.match(getBootstrapBusyNotice(), /能力安装\/校验/);
assert.match(getBootstrapBusyNotice(), /几十秒/);
assert.equal(getThinkingMascotAriaLabel("bootstrap"), "梦星星和星梦梦正在处理初始化协作包");
assert.equal(getBootstrapProgressIcon("running"), "thinking");
assert.equal(getWorkspacePathChipText(" E:/test/test-shop "), "E:/test/test-shop");
assert.equal(getWorkspacePathChipText("   "), null);
assert.equal(GITHUB_PROFILE_URL, "https://github.com/xinmengmeng-ai");
assert.equal(GITHUB_PROFILE_TOOLTIP, "打开 GitHub：xinmengmeng-ai");
assert.equal(CUSTOMER_SERVICE_CONTACT, "3972679968@qq.com");
assert.equal(CUSTOMER_SERVICE_TOOLTIP, "复制客服 QQ：3972679968@qq.com");
assert.equal(CUSTOMER_SERVICE_COPIED_TOOLTIP, "已复制客服 QQ");
assert.equal(WORK_IN_PROGRESS_TEXT, "星星正在加班开发ing~");
assert.equal(shouldRenderProviderCard({ providerId: "deepseek" }), true);
assert.equal(shouldRenderProviderCard({ providerId: "codex" }), true);
assert.equal(shouldRenderProviderCard({ providerId: "custom" }), true);
assert.equal(shouldRenderProviderCard({ providerId: "claude-code" }), true);
assert.equal(shouldRenderProviderCard({ providerId: "gemini-cli" }), true);
assert.equal(shouldDisableProviderCard({ providerId: "deepseek", blockingErrors: [] }), false);
assert.equal(
  shouldDisableProviderCard({
    providerId: "claude-code",
    blockingErrors: ["not_in_first_wave"],
  }),
  true,
);
assert.equal(
  shouldDisableProviderCard({
    providerId: "gemini-cli",
    blockingErrors: ["not_in_first_wave"],
  }),
  true,
);
assert.equal(
  getProviderAvailabilityText({
    providerId: "gemini-cli",
    blockingErrors: ["not_in_first_wave"],
    configDetected: false,
  }),
  WORK_IN_PROGRESS_TEXT,
);
assert.equal(
  getProviderAvailabilityText({
    providerId: "codex",
    blockingErrors: [],
    configDetected: true,
    authMode: "unsupported_official_login",
  }),
  "官方授权暂不支持",
);
assert.deepEqual(
  getTargetClientOptions().map((option) => ({
    id: option.id,
    disabled: option.disabled,
    disabledReason: option.disabledReason,
  })),
  [
    { id: "codex", disabled: false, disabledReason: undefined },
    { id: "claude-code", disabled: true, disabledReason: WORK_IN_PROGRESS_TEXT },
  ],
);
assert.equal(shouldRenderDiagnosticLogs([]), false);
assert.equal(shouldRenderDiagnosticLogs([""]), false);
assert.equal(shouldRenderDiagnosticLogs(["[diagnostic:error] detail=agent_response_invalid"]), true);
assert.deepEqual(
  createBootstrapProgressItems(
    [
      {
        id: "superpowers",
        label: "superpowers / required skills",
        selected: true,
        required: true,
        locked: true,
        detail: "skill_presence",
      },
    ],
    "codex",
  ).map((item) => ({
    label: item.label,
    status: item.status,
    motion: item.motion,
  })),
  [
    { label: "记录能力选择", status: "running", motion: true },
    { label: "准备目标软件路径", status: "pending", motion: false },
    { label: "执行能力安装/校验策略", status: "pending", motion: false },
    { label: "星梦梦语义验收", status: "pending", motion: false },
    { label: "生成初始化协作包并 postcheck", status: "pending", motion: false },
  ],
);
assert.equal(shouldShowSolutionDialog(null), false);
assert.equal(
  shouldShowSolutionDialog({
    stage: "conversation",
    solutions: [],
    finished: false,
  }),
  false,
);
assert.equal(
  shouldShowSolutionDialog({
    stage: "solutions_ready",
    solutions: [
      {
        id: "A",
      },
    ],
    finished: false,
  }),
  true,
);
assert.equal(
  shouldShowSolutionDialog({
    stage: "bootstrapping",
    solutions: [
      {
        id: "A",
      },
    ],
    toolCalls: [
      {
        toolName: "open_solution_selector",
        status: "requested",
      },
    ],
    finished: false,
  }),
  false,
);

const success = initializeSucceeded(workspace, {
  status: "success",
  stage: "postcheck",
  message: "postcheck passed",
  handoffPath: "E:/Projects/Product/.commonhe/session/bootstrap-handoff.md",
  logs: ["doctor passed", "postcheck passed"],
});

assert.equal(success.step, "handoff");
assert.equal(success.outcome, "success");
assert.ok(success.logs.includes("postcheck passed"));

const failure = initializeFailed(workspace, new Error("postcheck failed"));

assert.equal(failure.step, "handoff");
assert.equal(failure.outcome, "failure");
assert.match(failure.errorMessage ?? "", /postcheck failed/);

const discoveryResult = {
  Stage: "discovery",
  QuestionId: "project_name",
  QuestionText: "我们先定个名字。你想把这个项目叫什么？",
};

assert.equal(isFinalInitializationResult(discoveryResult), false);
assert.equal(isContinuableInitializationResult(discoveryResult), true);
assert.throws(
  () => buildInitializationSuccessResult(discoveryResult),
  /初始化尚未完成.*discovery.*project_name/s,
);

const discoveryError = buildInitializationBlockError(discoveryResult);
assert.match(discoveryError.message, /初始化尚未完成/);
assert.match(discoveryError.message, /project_name/);

const postcheckFailedResult = {
  Stage: "postcheck_failed",
  Message: "postcheck 未通过",
  Postcheck: { Passed: false },
};

assert.equal(isFinalInitializationResult(postcheckFailedResult), false);
assert.equal(isContinuableInitializationResult(postcheckFailedResult), false);
assert.throws(
  () => buildInitializationSuccessResult(postcheckFailedResult),
  /postcheck_failed.*postcheck 未通过/s,
);

assert.equal(isContinuableInitializationResult({ Stage: "proposal" }), true);
assert.equal(isContinuableInitializationResult({ Stage: "confirmed" }), true);

const finalResult = buildInitializationSuccessResult({
  Stage: "implementation_ready",
  Message: "初始化成功：postcheck 已通过。当前项目已推进到首个实施阶段。",
  HandoffPath: "E:/Projects/Product/.commonhe/session/bootstrap-handoff.md",
  Postcheck: { Passed: true },
});

assert.equal(finalResult.status, "success");
assert.equal(finalResult.stage, "implementation_ready");
assert.equal(finalResult.message, "初始化成功：postcheck 已通过。当前项目已推进到首个实施阶段。");
assert.ok(finalResult.logs.includes("postcheck passed"));
assert.equal(isContinuableInitializationResult(finalResult), false);

const proposalDisplay = extractProposalDisplay({
  Stage: "proposal",
  Recommended: "B",
  RecommendedOption: {
    id: "B",
    name: "平衡型方案",
    positioning: "兼顾交付速度和协作治理",
    time_cost: "中",
    deployment_cost: "低中",
    development_difficulty: "中",
    scalability: "中高",
    risks: "前期文档和测试工作量更高",
    enabled_roles: ["backend", "frontend", "qa"],
    recommendation_reason: "学生管理系统需要兼顾稳定和可验证。",
  },
  Options: [
    { id: "A", name: "快速 MVP 方案" },
    { id: "B", name: "平衡型方案" },
    { id: "C", name: "企业扩展型方案" },
  ],
});

assert.equal(proposalDisplay.recommendedId, "B");
assert.equal(proposalDisplay.options.length, 3);
assert.equal(proposalDisplay.options[1].id, "B");
assert.match(proposalDisplay.options[1].recommendationReason ?? "", /学生管理系统/);

assert.doesNotThrow(() =>
  validateInitializationStageRequest({
    stage: "bootstrap",
    sessionRoot: "E:/Projects/Product/.commonhe/session",
    projectRoot: "E:/Projects/Product",
    targetRoot: "E:/Projects/Product",
    projectName: "Product协作包",
    targetClient: "codex",
    execute: true,
  }),
);
assert.throws(
  () => validateInitializationStageRequest({ stage: "bootstrap", sessionRoot: "E:/x/.commonhe/session" }),
  /targetRoot is required/,
);
assert.throws(
  () => validateInitializationStageRequest({ stage: "answer", projectRoot: "E:/x", inputText: "answer" }),
  /sessionRoot is required/,
);
assert.doesNotThrow(() =>
  validateInitializationStageRequest({
    stage: "bootstrap",
    sessionRoot: "E:/Projects/Product/.commonhe/session",
    targetRoot: "E:/Projects/Product",
    choice: "B",
    projectName: "学生管理协作包",
    targetClient: "codex",
    selectedCapabilities: ["superpowers", "chrome-devtools"],
    execute: true,
  }),
);
assert.throws(
  () =>
    validateInitializationStageRequest({
      stage: "bootstrap",
      sessionRoot: "E:/Projects/Product/.commonhe/session",
      targetRoot: "E:/Projects/Product",
      choice: "B",
      projectName: "   ",
      targetClient: "codex",
      selectedCapabilities: ["superpowers"],
    }),
  /projectName is required/,
);
assert.throws(
  () =>
    validateInitializationStageRequest({
      stage: "bootstrap",
      sessionRoot: "E:/Projects/Product/.commonhe/session",
      targetRoot: "E:/Projects/Product",
      choice: "B",
      projectName: "学生管理协作包",
      targetClient: "cursor",
      selectedCapabilities: ["superpowers"],
    }),
  /targetClient must be codex or claude-code/,
);
