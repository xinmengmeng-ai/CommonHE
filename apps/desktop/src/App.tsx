import {
  AlertTriangle,
  CheckCircle2,
  FolderOpen,
  Headset,
  MessageSquareText,
  Play,
  RefreshCw,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useMemo, useReducer, useRef, useState } from "react";
import {
  FINAL_SUCCESS_TEXT,
  WORKFLOW_STEPS,
  buildMandatorySelectedCapabilities,
  createInitialState,
  formatRecoverableRuntimeErrorForUser,
  reducer,
  type CapabilityResult,
} from "./appState";
import {
  PRODUCT_LOGO_ALT_TEXT,
  advanceBootstrapProgress,
  completeBootstrapProgress,
  createBootstrapProgressItems,
  CUSTOMER_SERVICE_CONTACT,
  CUSTOMER_SERVICE_COPIED_TOOLTIP,
  CUSTOMER_SERVICE_TOOLTIP,
  GITHUB_PROFILE_TOOLTIP,
  GITHUB_PROFILE_URL,
  getProviderAvailabilityText,
  getBootstrapBusyNotice,
  getBootstrapButtonLabel,
  getBootstrapProgressIcon,
  getConversationStatusView,
  getSendButtonViewLabel,
  getTargetClientOptions,
  getThinkingMascotAriaLabel,
  getWorkspacePathChipText,
  shouldDisableProviderCard,
  shouldRenderProviderCard,
  shouldShowSolutionDialog,
  shouldRenderDiagnosticLogs,
  STAR_NAME,
  type BootstrapProgressItem,
} from "./agentFlow";
import {
  chooseAgentSolution,
  createAgentSession,
  discoverProviderModels,
  listProviderCatalog,
  loadProviderTools,
  locatePayload,
  openExternalUrl,
  pickWorkspaceDirectory,
  runOrchestratorStage,
  sendAgentMessage,
  validateProviderConfig,
  validateProviderConnection,
  type AgentSessionSnapshot,
  type AgentSolution,
  type ProviderCatalogEntry,
  type ProviderValidationResponse,
  type SelectedCapability,
  type TargetClient,
} from "./tauriApi";
import "./styles.css";
import brandLogoUrl from "./assets/xingmeng-logo.jpg";

function buildDefaultCapabilities(targetClient: TargetClient): SelectedCapability[] {
  return buildMandatorySelectedCapabilities(targetClient);
}

const DEFAULT_CAPABILITIES = buildDefaultCapabilities("codex");

type SolutionDialogStep = "solution" | "project" | "client" | "capabilities" | "installing";

const BOOTSTRAP_PROGRESS_DELAY_MS = 900;

function GitHubMark({ size = 18 }: { size?: number }) {
  return (
    <svg
      aria-hidden="true"
      fill="currentColor"
      height={size}
      viewBox="0 0 16 16"
      width={size}
    >
      <path d="M8 0C3.58 0 0 3.67 0 8.2c0 3.62 2.29 6.69 5.47 7.78.4.08.55-.18.55-.39 0-.19-.01-.84-.01-1.53-2.01.38-2.53-.5-2.69-.96-.09-.24-.48-.96-.82-1.15-.28-.16-.68-.55-.01-.56.63-.01 1.08.59 1.23.84.72 1.24 1.87.89 2.33.68.07-.53.28-.89.51-1.09-1.78-.21-3.64-.91-3.64-4.03 0-.89.31-1.62.82-2.19-.08-.21-.36-1.04.08-2.16 0 0 .67-.22 2.2.84A7.37 7.37 0 0 1 8 4.01c.68 0 1.36.09 2 .27 1.53-1.06 2.2-.84 2.2-.84.44 1.12.16 1.95.08 2.16.51.57.82 1.3.82 2.19 0 3.13-1.87 3.82-3.65 4.03.29.26.54.75.54 1.51 0 1.09-.01 1.97-.01 2.24 0 .21.15.47.55.39A8.09 8.09 0 0 0 16 8.2C16 3.67 12.42 0 8 0Z" />
    </svg>
  );
}

function ThinkingMascot({
  context,
  size = "small",
}: {
  context: "conversation" | "bootstrap";
  size?: "tiny" | "small" | "large";
}) {
  return (
    <span
      aria-label={getThinkingMascotAriaLabel(context)}
      className={`thinking-mascot is-${size}`}
      role="img"
    >
      <span className="mascot-orbit">
        <span />
        <span />
        <span />
      </span>
      <span className="mascot-figure">
        <span className="mascot-hair" />
        <span className="mascot-face">
          <span className="mascot-eye left" />
          <span className="mascot-eye right" />
          <span className="mascot-smile" />
        </span>
        <span className="mascot-headset" />
      </span>
      <span className="mascot-cube" />
    </span>
  );
}

function capabilityStatusFromStage(stage: string): CapabilityResult[] {
  return [
    "superpowers",
    "agent-browser",
    "chrome-devtools",
    "GitNexus",
    "Speckit",
  ].map((name) => ({
    name,
    status: stage === "doctor" || stage === "implementation_ready" ? "pass" : "unknown",
    detail: "doctor gate",
  }));
}

function buildSuccessMessage(solution: AgentSolution | undefined): string {
  return solution
    ? `初始化成功，已确认方案 ${solution.id}：${solution.title}。请在新会话中继续实施。`
    : "初始化成功，方案已确认。请在新会话中继续实施。";
}

function inferProjectName(snapshot: AgentSessionSnapshot): string {
  const source =
    snapshot.readiness.productType ??
    snapshot.understandingSummary ??
    snapshot.readiness.keyFeatures[0] ??
    "初始化协作包";
  const compact = source
    .replace(/[<>:"/\\|?*]/g, "")
    .replace(/\s+/g, "")
    .slice(0, 24);
  return compact ? `${compact}协作包` : "初始化协作包";
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

export default function App() {
  const [state, dispatch] = useReducer(reducer, undefined, createInitialState);
  const activeOperationRef = useRef<null | "session" | "message" | "bootstrap">(null);
  const [busyPhase, setBusyPhase] = useState<
    null | "scan" | "provider" | "model-discovery" | "session" | "message" | "bootstrap"
  >(null);
  const [providers, setProviders] = useState<ProviderCatalogEntry[]>([]);
  const [selectedProviderId, setSelectedProviderId] = useState<string>("deepseek");
  const [providerValidation, setProviderValidation] = useState<ProviderValidationResponse | null>(null);
  const [selectedModel, setSelectedModel] = useState("");
  const [useCustomModel, setUseCustomModel] = useState(false);
  const [customModel, setCustomModel] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  const [workspacePath, setWorkspacePath] = useState("");
  const [agentSession, setAgentSession] = useState<AgentSessionSnapshot | null>(null);
  const [agentInput, setAgentInput] = useState("");
  const [selectedSolutionId, setSelectedSolutionId] = useState("");
  const [customDiscoveredModels, setCustomDiscoveredModels] = useState<
    ProviderCatalogEntry["discoveredModels"]
  >([]);
  const [solutionDialogStep, setSolutionDialogStep] = useState<SolutionDialogStep>("solution");
  const [projectName, setProjectName] = useState("");
  const [targetClient, setTargetClient] = useState<TargetClient>("codex");
  const [selectedCapabilities, setSelectedCapabilities] =
    useState<SelectedCapability[]>(DEFAULT_CAPABILITIES);
  const [bootstrapProgressItems, setBootstrapProgressItems] = useState<BootstrapProgressItem[]>([]);
  const [customerServiceCopied, setCustomerServiceCopied] = useState(false);

  const currentIndex = useMemo(
    () => WORKFLOW_STEPS.findIndex((step) => step.id === state.step),
    [state.step],
  );
  const busy = busyPhase !== null;
  const visibleProviders = useMemo(
    () => providers.filter((provider) => shouldRenderProviderCard(provider)),
    [providers],
  );
  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.providerId === selectedProviderId) ?? providers[0],
    [providers, selectedProviderId],
  );
  const unsupportedOfficialCodexSelected =
    selectedProvider?.providerId === "codex" && selectedProvider.authMode === "unsupported_official_login";
  const availableModels = useMemo(() => {
    if (!selectedProvider) {
      return [];
    }
    if (selectedProvider.providerId === "custom" && customDiscoveredModels.length > 0) {
      return customDiscoveredModels;
    }
    return selectedProvider.discoveredModels;
  }, [customDiscoveredModels, selectedProvider]);
  const effectiveModel = useMemo(() => {
    if (useCustomModel) {
      return customModel.trim();
    }
    return selectedModel.trim();
  }, [customModel, selectedModel, useCustomModel]);
  const currentSolutions = agentSession?.solutions ?? [];
  const targetClientOptions = getTargetClientOptions();
  const solutionSelectorToolCall = agentSession?.toolCalls.find(
    (toolCall) => toolCall.toolName === "open_solution_selector",
  );
  const selectedSolution =
    currentSolutions.find((solution) => solution.id === selectedSolutionId) ?? currentSolutions[0];
  const showSolutionDialog = shouldShowSolutionDialog(agentSession);
  const conversationBusy = busyPhase === "session" || busyPhase === "message";
  const bootstrapBusy = busyPhase === "bootstrap";
  const busyNotice = bootstrapBusy
    ? getBootstrapBusyNotice()
    : conversationBusy
      ? "梦星星正在思考并整理下一轮回复，请稍候，窗口会保持可响应。"
      : null;
  const conversationStatusView = getConversationStatusView({
    busyNotice,
    recoverableError: state.recoverableError,
  });
  const workspacePathChipText = getWorkspacePathChipText(state.workspace?.path);

  function beginOperation(operation: "session" | "message" | "bootstrap"): boolean {
    if (activeOperationRef.current) {
      dispatch({
        type: "initialize.log",
        payload: "上一轮模型请求仍在收尾，已忽略重复触发；请等待按钮恢复后再继续。",
      });
      return false;
    }
    dispatch({ type: "initialize.recoverableError", payload: undefined });
    activeOperationRef.current = operation;
    setBusyPhase(operation);
    return true;
  }

  function endOperation(operation: "session" | "message" | "bootstrap") {
    if (activeOperationRef.current === operation) {
      activeOperationRef.current = null;
      setBusyPhase(null);
    }
  }

  function logRecoverableRuntimeError(operation: "session" | "message" | "bootstrap", error: unknown) {
    const message = error instanceof Error ? error.message : String(error);
    const userMessage = formatRecoverableRuntimeErrorForUser(message, operation);
    dispatch({
      type: "initialize.log",
      payload: userMessage,
    });
    dispatch({ type: "initialize.recoverableError", payload: userMessage });
    dispatch({
      type: "initialize.diagnostic",
      payload: {
        operation,
        severity: "error",
        timestamp: new Date().toISOString(),
        provider: selectedProvider?.providerId,
        model: providerValidation?.resolvedModel ?? effectiveModel,
        baseUrl: providerValidation?.resolvedBaseUrl ?? baseUrl,
        wireApi: providerValidation?.resolvedWireApi,
        detail: message,
      },
    });
  }

  function resetAll() {
    dispatch({ type: "reset" });
    setProviders([]);
    setSelectedProviderId("deepseek");
    setProviderValidation(null);
    setSelectedModel("");
    setUseCustomModel(false);
    setCustomModel("");
    setApiKey("");
    setBaseUrl("");
    setWorkspacePath("");
    setAgentSession(null);
    setAgentInput("");
    setSelectedSolutionId("");
    setCustomDiscoveredModels([]);
    setSolutionDialogStep("solution");
    setProjectName("");
    setTargetClient("codex");
    setSelectedCapabilities(buildDefaultCapabilities("codex"));
    setBootstrapProgressItems([]);
    setCustomerServiceCopied(false);
  }

  async function copyCustomerServiceContact() {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(CUSTOMER_SERVICE_CONTACT);
      } else {
        const textArea = document.createElement("textarea");
        textArea.value = CUSTOMER_SERVICE_CONTACT;
        textArea.setAttribute("readonly", "true");
        textArea.style.position = "fixed";
        textArea.style.left = "-9999px";
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand("copy");
        document.body.removeChild(textArea);
      }
      setCustomerServiceCopied(true);
      window.setTimeout(() => setCustomerServiceCopied(false), 1600);
    } catch (error) {
      dispatch({
        type: "initialize.log",
        payload: `客服 QQ 复制失败：${error instanceof Error ? error.message : String(error)}`,
      });
    }
  }

  async function openGitHubProfile() {
    try {
      await openExternalUrl(GITHUB_PROFILE_URL);
    } catch (error) {
      dispatch({
        type: "initialize.log",
        payload: `GitHub 打开失败：${error instanceof Error ? error.message : String(error)}`,
      });
    }
  }

  function selectTargetClient(nextTargetClient: TargetClient) {
    const nextTargetClientOption = targetClientOptions.find((option) => option.id === nextTargetClient);
    if (!nextTargetClientOption || nextTargetClientOption.disabled) {
      return;
    }
    setTargetClient(nextTargetClient);
    setSelectedCapabilities(buildDefaultCapabilities(nextTargetClient));
  }

  function resetProviderState(provider: ProviderCatalogEntry) {
    setSelectedProviderId(provider.providerId);
    setProviderValidation(null);
    setApiKey("");
    setBaseUrl(provider.defaultBaseUrl ?? "");
    setCustomDiscoveredModels([]);
    setUseCustomModel(false);
    setCustomModel("");
    setSelectedModel(provider.defaultModel || provider.discoveredModels[0]?.id || "");
  }

  async function scanEnvironment() {
    setBusyPhase("scan");
    try {
      const payload = await locatePayload();
      const providerCatalog = await listProviderCatalog();
      setProviders(providerCatalog);
      const visibleProviderCatalog = providerCatalog.filter((provider) => shouldRenderProviderCard(provider));
      const preferred =
        visibleProviderCatalog.find(
          (provider) =>
            !provider.blockingErrors.includes("not_in_first_wave") && provider.configDetected,
        ) ??
        visibleProviderCatalog.find((provider) => provider.providerId === "deepseek") ??
        visibleProviderCatalog[0];

      if (preferred) {
        resetProviderState(preferred);
        const details = await loadProviderTools(preferred.providerId);
        dispatch({
          type: "initialize.log",
          payload: `渠道已就绪：${details.label}（${getProviderAvailabilityText(details)}）`,
        });
      }

      const doctor = await runOrchestratorStage({ stage: "doctor", projectRoot: payload.payloadRoot });
      dispatch({
        type: "scan.complete",
        payload: {
          payloadPath: payload.payloadRoot,
          capabilities: capabilityStatusFromStage(doctor.Stage ?? doctor.stage ?? "doctor"),
        },
      });
      const configuredProviders = providerCatalog
        .filter((provider) => provider.configDetected)
        .map((provider) => provider.providerId)
        .join(", ");
      if (configuredProviders) {
        dispatch({
          type: "initialize.log",
          payload: `已发现本地配置：${configuredProviders}`,
        });
      }
    } catch (error) {
      dispatch({ type: "scan.complete", payload: { payloadPath: "not found", capabilities: [] } });
      dispatch({ type: "initialize.log", payload: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyPhase(null);
    }
  }

  async function selectProvider(provider: ProviderCatalogEntry) {
    if (shouldDisableProviderCard(provider)) {
      return;
    }
    resetProviderState(provider);
    dispatch({
      type: "initialize.log",
      payload: `已选择渠道：${provider.label}`,
    });
  }

  async function loadCustomModels() {
    if (!selectedProvider || selectedProvider.providerId !== "custom") {
      return;
    }

    setBusyPhase("model-discovery");
    try {
      const models = await discoverProviderModels({
        provider: "custom",
        apiKey,
        baseUrl,
      });
      setCustomDiscoveredModels(models);
      if (models[0]) {
        setSelectedModel(models[0].id);
        setUseCustomModel(false);
      }
      dispatch({
        type: "initialize.log",
        payload: `已发现 Custom 模型：${models.map((model) => model.id).join("、")}`,
      });
    } catch (error) {
      dispatch({
        type: "initialize.log",
        payload: error instanceof Error ? error.message : String(error),
      });
    } finally {
      setBusyPhase(null);
    }
  }

  async function confirmProvider() {
    if (!selectedProvider) {
      return;
    }

    setBusyPhase("provider");
    try {
      const config = {
        provider: selectedProvider.providerId,
        model: effectiveModel,
        apiKey,
        baseUrl,
      };
      const prepared = await validateProviderConfig(config);
      if (!prepared.valid) {
        setProviderValidation(prepared);
        throw new Error(prepared.userFacingError ?? `渠道校验失败：${prepared.errors.join(", ")}`);
      }
      const result = await validateProviderConnection(config);
      setProviderValidation(result);
      if (!result.valid) {
        throw new Error(result.userFacingError ?? `渠道连通性校验失败：${result.errors.join(", ")}`);
      }
      dispatch({
        type: "provider.validated",
        payload: {
          providerId: result.normalizedProvider,
          providerLabel: selectedProvider.label,
          model: result.resolvedModel ?? effectiveModel,
        },
      });
    } catch (error) {
      dispatch({ type: "initialize.log", payload: error instanceof Error ? error.message : String(error) });
    } finally {
      setBusyPhase(null);
    }
  }

  async function pickWorkspace() {
    const selected = await pickWorkspaceDirectory();
    if (selected) {
      setWorkspacePath(selected);
    }
  }

  function confirmWorkspace() {
    const trimmed = workspacePath.trim();
    if (!trimmed) {
      dispatch({ type: "initialize.log", payload: "请选择工作区路径。" });
      return;
    }

    dispatch({
      type: "workspace.selected",
      payload: {
        path: trimmed,
        kind: "unknown",
      signal: "梦星星会在该工作区上下文中运行。",
      },
    });
    setAgentSession(null);
    setAgentInput("");
    setSelectedSolutionId("");
  }

  function returnToProviderSettings() {
    if (busy) {
      return;
    }

    setAgentSession(null);
    setAgentInput("");
    setSelectedSolutionId("");
    setProjectName("");
    setSolutionDialogStep("solution");
    setBootstrapProgressItems([]);
    setProviderValidation(null);
    dispatch({ type: "provider.reselect_requested" });
  }

  async function startAgentSessionFlow() {
    if (!state.workspace?.path || !selectedProvider || !providerValidation?.valid) {
      return;
    }
    if (!beginOperation("session")) {
      return;
    }

    try {
      const snapshot = await createAgentSession({
        provider: selectedProvider.providerId,
        model: providerValidation.resolvedModel ?? effectiveModel,
        apiKey,
        baseUrl: providerValidation.resolvedBaseUrl ?? baseUrl,
        wireApi: providerValidation.resolvedWireApi,
        workspacePath: state.workspace.path,
      });
      setAgentSession(snapshot);
      if (snapshot.solutions[0]) {
        setSelectedSolutionId(snapshot.solutions[0].id);
      }
      dispatch({ type: "initialize.started" });
      dispatch({ type: "initialize.log", payload: "梦星星会话已建立。" });
    } catch (error) {
      logRecoverableRuntimeError("session", error);
    } finally {
      endOperation("session");
    }
  }

  async function submitAgentMessage() {
    if (!agentSession?.sessionId || !agentInput.trim()) {
      return;
    }
    if (!beginOperation("message")) {
      return;
    }

    try {
      const snapshot = await sendAgentMessage({
        sessionId: agentSession.sessionId,
        message: agentInput.trim(),
      });
      setAgentSession(snapshot);
      if (snapshot.solutions[0]) {
        setSelectedSolutionId(snapshot.solutions[0].id);
      }
      setAgentInput("");
      if (snapshot.solutions.length > 0 && !projectName.trim()) {
        setProjectName(inferProjectName(snapshot));
      }
    } catch (error) {
      logRecoverableRuntimeError("message", error);
    } finally {
      endOperation("message");
    }
  }

  async function confirmSolution() {
    if (!agentSession?.sessionId || !selectedSolutionId || !projectName.trim()) {
      return;
    }

    const progressItems = createBootstrapProgressItems(selectedCapabilities, targetClient);
    if (!beginOperation("bootstrap")) {
      return;
    }
    setSolutionDialogStep("installing");
    setBootstrapProgressItems(progressItems);
    dispatch({
      type: "initialize.log",
      payload: `开始能力处理：${selectedCapabilities
        .filter((capability) => capability.selected)
        .map((capability) => capability.label)
        .join("、") || "未选择可选能力"}`,
    });
    try {
      await wait(BOOTSTRAP_PROGRESS_DELAY_MS);
      setBootstrapProgressItems((items) => advanceBootstrapProgress(items));
      dispatch({ type: "initialize.log", payload: "已准备目标软件入口与配置路径。" });

      await wait(BOOTSTRAP_PROGRESS_DELAY_MS);
      setBootstrapProgressItems((items) => advanceBootstrapProgress(items));
      dispatch({ type: "initialize.log", payload: "正在执行能力安装/校验策略，并记录 fallback/blocked 状态。" });

      await wait(BOOTSTRAP_PROGRESS_DELAY_MS);
      setBootstrapProgressItems((items) => advanceBootstrapProgress(items));
      dispatch({ type: "initialize.log", payload: "星梦梦正在进行语义验收；发现阻断项会回传梦星星修正。" });

      const [snapshot] = await Promise.all([
        chooseAgentSolution({
          sessionId: agentSession.sessionId,
          solutionId: selectedSolutionId,
          projectName: projectName.trim(),
          targetClient,
          selectedCapabilities,
        }),
        wait(BOOTSTRAP_PROGRESS_DELAY_MS),
      ]);
      setBootstrapProgressItems((items) => completeBootstrapProgress(advanceBootstrapProgress(items)));
      setAgentSession(snapshot);
      if (snapshot.bootstrapResult?.status === "success") {
        dispatch({
          type: "initialize.succeeded",
          payload: {
            status: "success",
            stage: snapshot.stage,
            message: snapshot.bootstrapResult.userFacingMessage ?? buildSuccessMessage(selectedSolution),
            logs: [
              "渠道验证已完成",
              "梦星星已完成需求澄清和三方案输出",
              "星梦梦语义验收已通过",
              "已完成方案选择与初始化落盘",
            ],
          },
        });
      } else {
        throw new Error(
          snapshot.bootstrapResult?.userFacingMessage ??
            (snapshot.semanticReviewIssues.length > 0
              ? snapshot.semanticReviewIssues.join("；")
              : "初始化未能生成真实工作区内容，已阻断成功收口。"),
        );
      }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (message.includes("模型请求失败") || message.includes("error sending request for url")) {
        logRecoverableRuntimeError("bootstrap", error);
        setSolutionDialogStep("capabilities");
      } else {
        dispatch({ type: "initialize.failed", payload: error });
      }
    } finally {
      endOperation("bootstrap");
    }
  }

  return (
    <main className="shell">
      <aside className="rail" aria-label="workflow">
        <div className="mark">
          <img alt={PRODUCT_LOGO_ALT_TEXT} src={brandLogoUrl} />
        </div>
        {WORKFLOW_STEPS.map((step, index) => (
          <div
            className={`rail-step ${index === currentIndex ? "is-current" : ""} ${
              index < currentIndex ? "is-done" : ""
            }`}
            key={step.id}
          >
            <span>{index + 1}</span>
            <strong>{step.label}</strong>
          </div>
        ))}
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div className="brand-lockup">
            <img alt="" aria-hidden="true" className="brand-logo" src={brandLogoUrl} />
            <div>
              <p className="eyebrow">星星的vibecoding启动器</p>
              <h1>让梦星星先把想法聊明白</h1>
            </div>
          </div>
          <div className="topbar-actions">
            {busyNotice ? (
              <div aria-live="polite" className="busy-chip" role="status">
                <ThinkingMascot context={bootstrapBusy ? "bootstrap" : "conversation"} size="tiny" />
                <span>{busyNotice}</span>
              </div>
            ) : null}
            <button
              aria-label={GITHUB_PROFILE_TOOLTIP}
              className="icon-button"
              onClick={() => void openGitHubProfile()}
              title={GITHUB_PROFILE_TOOLTIP}
              type="button"
            >
              <GitHubMark size={18} />
            </button>
            <button
              aria-label={customerServiceCopied ? CUSTOMER_SERVICE_COPIED_TOOLTIP : CUSTOMER_SERVICE_TOOLTIP}
              className="icon-button"
              onClick={() => void copyCustomerServiceContact()}
              title={customerServiceCopied ? CUSTOMER_SERVICE_COPIED_TOOLTIP : CUSTOMER_SERVICE_TOOLTIP}
              type="button"
            >
              <Headset size={18} />
            </button>
            <button className="icon-button" onClick={resetAll} title="重新开始">
              <RefreshCw size={18} />
            </button>
          </div>
        </header>

        {state.step === "scan" && (
          <section className="stage">
            <div className="stage-copy">
              <ShieldCheck size={36} />
              <h2>环境准备</h2>
              <p>先确认本地能力、资源包和首批正式渠道，再进入和梦星星的正式对话。</p>
            </div>
            <button className="primary" disabled={busy} onClick={scanEnvironment}>
              <Play size={18} />
              开始检测
            </button>
          </section>
        )}

        {state.step === "provider" && (
          <section className="stage stage-with-fixed-action">
            <div className="stage-scroll">
              <div className="provider-grid">
                {visibleProviders.map((provider) => (
                  <button
                    className={`provider ${selectedProvider?.providerId === provider.providerId ? "is-selected" : ""} ${
                      shouldDisableProviderCard(provider) ? "is-disabled" : ""
                    }`}
                    disabled={shouldDisableProviderCard(provider)}
                    key={provider.providerId}
                    onClick={() => void selectProvider(provider)}
                    type="button"
                  >
                    <strong>{provider.label}</strong>
                    <span>{getProviderAvailabilityText(provider)}</span>
                  </button>
                ))}
              </div>

              {selectedProvider && (
                <>
                  <div className="tool-load-status">
                    <strong>渠道状态：{selectedProvider.label}</strong>
                    <span>配置发现：{selectedProvider.configDetected ? "已发现" : "未发现"}</span>
                    <span>模型发现：{availableModels.length > 0 ? `${availableModels.length} 个` : "待发现"}</span>
                    <span>
                      鉴权状态：
                      {providerValidation?.authStatus === "official_login" || unsupportedOfficialCodexSelected
                        ? "官方 Codex 登录"
                        : providerValidation?.authStatus
                          ? providerValidation.authStatus
                          : selectedProvider.requiresApiKey
                            ? "待验证"
                            : "无需鉴权"}
                    </span>
                    <span>
                      渠道可调用：
                      {providerValidation?.connectivityValidated
                        ? "已验证"
                        : unsupportedOfficialCodexSelected
                          ? "不支持"
                          : "未验证"}
                    </span>
                    {selectedProvider.detectedSources.length > 0 && (
                      <small>{selectedProvider.detectedSources.join("；")}</small>
                    )}
                    {selectedProvider.userWarnings.length > 0 && (
                      <small>{selectedProvider.userWarnings.join("；")}</small>
                    )}
                  </div>

                  <div className="form-grid">
                    <label>
                      模型
                      {availableModels.length > 0 && !useCustomModel ? (
                        <select value={selectedModel} onChange={(event) => setSelectedModel(event.target.value)}>
                          {availableModels.map((model) => (
                            <option key={model.id} value={model.id}>
                              {model.label}
                            </option>
                          ))}
                        </select>
                      ) : (
                        <input
                          value={useCustomModel ? customModel : selectedModel}
                          onChange={(event) =>
                            useCustomModel
                              ? setCustomModel(event.target.value)
                              : setSelectedModel(event.target.value)
                          }
                          disabled={!useCustomModel && availableModels.length > 0}
                        />
                      )}
                    </label>

                    {unsupportedOfficialCodexSelected && (
                      <div className="official-provider-note">
                        当前检测到的是官方 Codex 登录授权，星星的vibecoding启动器暂不支持该线路。请改用 Codex 的 OpenAI Responses API 配置。
                      </div>
                    )}

                    {selectedProvider.requiresApiKey && !unsupportedOfficialCodexSelected && (
                      <label>
                        API Key
                        <input
                          value={apiKey}
                          type="password"
                          onChange={(event) => setApiKey(event.target.value)}
                          placeholder="不会回显密钥内容"
                        />
                      </label>
                    )}

                    {!unsupportedOfficialCodexSelected && (selectedProvider.requiresBaseUrl || selectedProvider.defaultBaseUrl) && (
                      <label>
                        Base URL
                        <input value={baseUrl} onChange={(event) => setBaseUrl(event.target.value)} />
                      </label>
                    )}
                  </div>

                  <div className="action-row">
                    {selectedProvider.supportsCustomModel && (
                      <button
                        className="secondary"
                        onClick={() => setUseCustomModel((current) => !current)}
                        type="button"
                      >
                        {useCustomModel ? "改回列表模型" : "使用自定义模型"}
                      </button>
                    )}
                    {selectedProvider.providerId === "custom" && (
                      <button className="secondary" disabled={busy} onClick={loadCustomModels} type="button">
                        <Sparkles size={18} />
                        读取模型列表
                      </button>
                    )}
                  </div>

                  {providerValidation?.blockingErrors.length ? (
                    <div className="error-box">
                      {providerValidation.userFacingError ?? providerValidation.blockingErrors.join("；")}
                    </div>
                  ) : null}
                </>
              )}
            </div>

            {selectedProvider && (
              <div className="stage-action-bar">
                <button
                  className="primary"
                  disabled={busy || selectedProvider.blockingErrors.length > 0}
                  onClick={confirmProvider}
                >
                  <CheckCircle2 size={18} />
                  确认渠道并验证
                </button>
              </div>
            )}
          </section>
        )}

        {state.step === "workspace" && (
          <section className="stage">
            <div className="form-grid workspace-form">
              <label>
                工作区
                <input value={workspacePath} onChange={(event) => setWorkspacePath(event.target.value)} />
              </label>
              <button className="secondary" onClick={pickWorkspace}>
                <FolderOpen size={18} />
                浏览
              </button>
            </div>
            <button className="primary" onClick={confirmWorkspace}>
              <CheckCircle2 size={18} />
              锁定工作区
            </button>
          </section>
        )}

        {state.step === "initialize" && (
          <section className="stage initialize-stage">
            {workspacePathChipText ? (
              <div className="workspace-path-chip" title={workspacePathChipText}>
                <MessageSquareText size={16} />
                <span>{workspacePathChipText}</span>
              </div>
            ) : null}

            {!agentSession && (
              <>
                <button
                  className="primary"
                  disabled={busy || !providerValidation?.valid}
                  onClick={startAgentSessionFlow}
                >
                  <Play size={18} />
                    {conversationBusy ? "梦星星正在就位..." : state.recoverableError ? "重试启动梦星星" : "启动梦星星"}
                </button>
                {conversationStatusView.kind === "busy" ? (
                  <div aria-live="polite" className="assistant-status" role="status">
                    <ThinkingMascot context="conversation" size="tiny" />
                    <span>{conversationStatusView.message}</span>
                  </div>
                ) : null}
                {conversationStatusView.kind === "error" ? (
                  <div className="action-row">
                    <button className="secondary" disabled={busy} onClick={returnToProviderSettings} type="button">
                      <RefreshCw size={18} />
                      返回渠道与模型设置
                    </button>
                  </div>
                ) : null}
              </>
            )}

            {agentSession && (
              <div className="conversation-layout">
                <div
                  aria-busy={conversationBusy || bootstrapBusy}
                  className="conversation-panel"
                >
                  {conversationStatusView.kind === "busy" ? (
                    <div aria-live="polite" className="assistant-status" role="status">
                      <ThinkingMascot context={bootstrapBusy ? "bootstrap" : "conversation"} size="tiny" />
                      <span>{conversationStatusView.message}</span>
                    </div>
                  ) : null}
                  {conversationStatusView.kind === "error" ? (
                    <div aria-live="assertive" className="assistant-status is-error" role="status">
                      <AlertTriangle size={16} />
                      <span>{conversationStatusView.message}</span>
                    </div>
                  ) : null}
                  {conversationStatusView.kind === "error" ? (
                    <div className="action-row">
                      <button className="secondary" disabled={busy} onClick={returnToProviderSettings} type="button">
                        <RefreshCw size={18} />
                        返回渠道与模型设置
                      </button>
                    </div>
                  ) : null}
                  <div className="conversation-log">
                    {agentSession.messages.map((message, index) => (
                      <article className={`message ${message.role}`} key={`${message.role}-${index}`}>
                        <strong>{message.role === "assistant" ? STAR_NAME : "你"}</strong>
                        <p>{message.content}</p>
                      </article>
                    ))}
                    {conversationBusy ? (
                      <article className="message assistant pending-message">
                        <strong>{STAR_NAME}</strong>
                        <p>
                          <ThinkingMascot context="conversation" size="small" />
                          我在整理你的需求和下一步问题，马上回来。
                        </p>
                      </article>
                    ) : null}
                  </div>

                  {agentSession.stage === "conversation" && (
                    <div className="composer">
                      <textarea
                        disabled={conversationBusy || bootstrapBusy}
                        rows={4}
                        value={agentInput}
                        onChange={(event) => setAgentInput(event.target.value)}
                  placeholder="告诉梦星星你想做的产品、目标用户、核心功能、偏好的实现形态"
                      />
                      <button
                        className="primary"
                        disabled={conversationBusy || bootstrapBusy || !agentInput.trim()}
                        onClick={submitAgentMessage}
                      >
                        <CheckCircle2 size={18} />
                        {getSendButtonViewLabel({
                          conversationBusy,
                          recoverableError: state.recoverableError,
                        })}
                      </button>
                    </div>
                  )}
                </div>

                <div className="solution-panel">
                  <h3>当前理解</h3>
                  <p>{agentSession.understandingSummary ?? "梦星星仍在澄清需求。"}</p>
                  <p>
                    当前仍需补充：
                    {agentSession.readiness.missingFields.length > 0
                      ? agentSession.readiness.missingFields.join("、")
                      : "已满足出方案条件"}
                  </p>

                  {currentSolutions.length > 0 && (
                    <>
                      <h3>方案状态</h3>
                      <p>
                    梦星星已提交 {currentSolutions.length} 套方案，
                        {showSolutionDialog
                          ? "内置方案选择弹出框已打开，请确认最终路线。"
                          : bootstrapBusy || agentSession.stage === "bootstrapping"
                            ? "正在把已选方案落成真实初始化结果。"
                            : "等待进入方案选择。"}
                      </p>
                      <p>
                        工具动作：
                        {solutionSelectorToolCall
                          ? `${solutionSelectorToolCall.toolName} / ${solutionSelectorToolCall.status}`
                          : "尚未请求"}
                      </p>
                    </>
                  )}
                </div>
              </div>
            )}

            {showSolutionDialog && (
              <div className="dialog-backdrop" role="presentation">
                <div
                  aria-labelledby="solution-dialog-title"
                  aria-modal="true"
                  className="solution-dialog"
                  role="dialog"
                >
                  <div className="solution-dialog-header">
                    <div>
                      <p className="eyebrow">星星的vibecoding启动器</p>
                      <h2 id="solution-dialog-title">
                        {solutionDialogStep === "solution"
                          ? "选择最终初始化方案"
                          : solutionDialogStep === "project"
                            ? "填写初始化协作包项目名"
                            : solutionDialogStep === "client"
                              ? "选择后续使用的软件"
                              : "确认必需工作流能力"}
                      </h2>
                    </div>
                    <p>
                      {solutionDialogStep === "solution"
                  ? "梦星星已经完成三方案整理，请在这里确认最适合当前产品目标的路径。"
                        : solutionDialogStep === "project"
                          ? "这个名字会写入初始化协作包真源，不再使用工作区文件夹名代替。"
                          : solutionDialogStep === "client"
                            ? "v1.0 当前先开放 Codex；Claude Code 仍在开发中。"
                  : "五项能力为当前 Agent 工作流必需项，不提供取消入口；梦星星会把安装/校验结果写入 session。"}
                    </p>
                  </div>

                  <div className="solution-dialog-body">
                    {solutionDialogStep === "solution" && (
                      <div className="proposal-grid">
                        {currentSolutions.map((solution) => (
                          <button
                            className={`proposal-card ${selectedSolutionId === solution.id ? "is-selected" : ""}`}
                            key={solution.id}
                            onClick={() => setSelectedSolutionId(solution.id)}
                            type="button"
                          >
                            <span>方案 {solution.id}</span>
                            <strong>{solution.title}</strong>
                            <p>{solution.architectureSummary}</p>
                            <p>{solution.teamComposition.join("、")}</p>
                            <p>{solution.tokenEstimate}</p>
                            <p>{solution.recommendationText}</p>
                          </button>
                        ))}
                      </div>
                    )}

                    {solutionDialogStep === "project" && (
                      <div className="form-grid">
                        <label>
                          初始化协作包项目名
                          <input
                            value={projectName}
                            onChange={(event) => setProjectName(event.target.value)}
                            placeholder="例如：学生管理协作包"
                          />
                        </label>
                      </div>
                    )}

                    {solutionDialogStep === "client" && (
                      <div className="proposal-grid compact-grid">
                        {targetClientOptions.map((option) => (
                          <button
                            className={`proposal-card ${targetClient === option.id ? "is-selected" : ""} ${
                              option.disabled ? "is-disabled" : ""
                            }`}
                            disabled={option.disabled}
                            key={option.id}
                            onClick={() => selectTargetClient(option.id)}
                            type="button"
                          >
                            <span>{option.label}</span>
                            <strong>{option.title}</strong>
                            <p>{option.description}</p>
                            {option.disabledReason ? <small>{option.disabledReason}</small> : null}
                          </button>
                        ))}
                      </div>
                    )}

                    {solutionDialogStep === "capabilities" && (
                      <div className="capability-choice-list">
                        {selectedCapabilities.map((capability) => (
                          <label className="capability-choice" key={capability.id}>
                            <input checked disabled type="checkbox" />
                            <span>
                              <strong>{capability.label}</strong>
                              <small>{capability.detail}</small>
                              <small>必需能力，当前版本不可取消。</small>
                            </span>
                          </label>
                        ))}
                      </div>
                    )}

                    {solutionDialogStep === "installing" && (
                      <div className="install-progress-panel">
                        <div className="install-hero">
                          <ThinkingMascot context="bootstrap" size="large" />
                          <div>
                            <strong>梦星星与星梦梦正在完成初始化协作包验收</strong>
                            <p>{getBootstrapBusyNotice()}</p>
                          </div>
                        </div>
                        <ul className="bootstrap-progress-list">
                          {bootstrapProgressItems.map((item) => {
                            const icon = getBootstrapProgressIcon(item.status);
                            return (
                              <li
                                className={`bootstrap-progress-item is-${item.status} ${
                                  item.motion ? "has-motion" : ""
                                }`}
                                key={item.label}
                              >
                                <span className="bootstrap-progress-icon">
                                  {icon === "thinking" ? (
                                    <ThinkingMascot context="bootstrap" size="tiny" />
                                  ) : (
                                    icon
                                  )}
                                </span>
                                <span className="bootstrap-progress-text">
                                  <strong>{item.label}</strong>
                                  <small>{item.detail}</small>
                                </span>
                              </li>
                            );
                          })}
                        </ul>
                      </div>
                    )}
                  </div>

                  <div className="solution-dialog-footer">
                    <p>
                      {solutionDialogStep === "solution" &&
                        `当前选择：${selectedSolution ? `${selectedSolution.id} - ${selectedSolution.title}` : "尚未选择"}`}
                      {solutionDialogStep === "project" && `项目名：${projectName.trim() || "尚未填写"}`}
                      {solutionDialogStep === "client" &&
                        `后续软件：${targetClient === "codex" ? "Codex" : "Claude Code"}`}
                      {solutionDialogStep === "capabilities" &&
                        `必需能力：${selectedCapabilities.length} 项，全部必选`}
                      {solutionDialogStep === "installing" &&
                        "正在真实执行能力安装/校验、语义验收和落盘，期间会保持动效与运行日志。"}
                    </p>
                    {solutionDialogStep !== "solution" && (
                      <button
                        className="secondary"
                        disabled={busy || solutionDialogStep === "installing"}
                        onClick={() =>
                          setSolutionDialogStep((current) =>
                            current === "capabilities"
                              ? "client"
                              : current === "client"
                                ? "project"
                                : "solution",
                          )
                        }
                        type="button"
                      >
                        返回上一步
                      </button>
                    )}
                    {solutionDialogStep === "solution" && (
                      <button
                        className="primary"
                        disabled={busy || !selectedSolutionId}
                        onClick={() => setSolutionDialogStep("project")}
                        type="button"
                      >
                        <CheckCircle2 size={18} />
                        继续填写项目名
                      </button>
                    )}
                    {solutionDialogStep === "project" && (
                      <button
                        className="primary"
                        disabled={busy || !projectName.trim()}
                        onClick={() => setSolutionDialogStep("client")}
                        type="button"
                      >
                        <CheckCircle2 size={18} />
                        继续选择软件
                      </button>
                    )}
                    {solutionDialogStep === "client" && (
                      <button
                        className="primary"
                        disabled={busy}
                        onClick={() => setSolutionDialogStep("capabilities")}
                        type="button"
                      >
                        <CheckCircle2 size={18} />
                        继续确认能力
                      </button>
                    )}
                    {solutionDialogStep === "capabilities" && (
                      <button className="primary" disabled={busy || !selectedSolutionId} onClick={confirmSolution}>
                        <CheckCircle2 size={18} />
                        {getBootstrapButtonLabel(bootstrapBusy)}
                      </button>
                    )}
                    {solutionDialogStep === "installing" && (
                      <button className="primary" disabled type="button">
                        <ThinkingMascot context="bootstrap" size="tiny" />
                        梦星星与星梦梦正在执行中
                      </button>
                    )}
                  </div>
                </div>
              </div>
            )}
          </section>
        )}

        {state.step === "handoff" && (
          <section className={`stage handoff ${state.outcome === "failure" ? "is-failure" : ""}`}>
            {state.outcome === "failure" ? <AlertTriangle size={40} /> : <CheckCircle2 size={40} />}
            <h2>{state.outcome === "failure" ? "初始化被阻断" : FINAL_SUCCESS_TEXT}</h2>
            <p>{state.result?.message ?? state.errorMessage}</p>
          </section>
        )}

        <section className="evidence">
          <div>
            <h3>能力门禁</h3>
            <div className="capability-list">
              {state.capabilities.map((capability) => (
                <span className={`capability ${capability.status}`} key={capability.name}>
                  {capability.name}
                </span>
              ))}
            </div>
          </div>
          <div>
            <h3>运行日志</h3>
            <pre>{state.logs.join("\n") || "等待开始检测"}</pre>
          </div>
          {shouldRenderDiagnosticLogs(state.diagnosticLogs) ? (
            <details className="diagnostic-details">
              <summary>诊断日志</summary>
              <p className="diagnostic-note">
                给排查用：记录阶段、渠道、模型、API Base URL 与底层错误；不会展示 APIKey。
              </p>
              <pre>{state.diagnosticLogs.join("\n")}</pre>
            </details>
          ) : null}
        </section>
      </section>
    </main>
  );
}
