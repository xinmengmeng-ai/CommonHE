export const STAR_NAME = "梦星星";
export const CONVERSATION_STAGE_TITLE = "和梦星星对话";
export const SEND_TO_STAR_LABEL = "发送给梦星星";
export const STAR_THINKING_LABEL = "梦星星思考中...";
export const STAR_BOOTSTRAPPING_LABEL = "星梦梦正在复核，梦星星正在整理初始化结果...";
export const PRODUCT_LOGO_ALT_TEXT = "星星的vibecoding启动器 logo";
export const GITHUB_PROFILE_URL = "https://github.com/xinmengmeng-ai";
export const GITHUB_PROFILE_TOOLTIP = "打开 GitHub：xinmengmeng-ai";
export const CUSTOMER_SERVICE_CONTACT = "3972679968@qq.com";
export const CUSTOMER_SERVICE_TOOLTIP = "复制客服 QQ：3972679968@qq.com";
export const CUSTOMER_SERVICE_COPIED_TOOLTIP = "已复制客服 QQ";
export const WORK_IN_PROGRESS_TEXT = "星星正在加班开发ing~";
export const AGENT_START_LOG =
  "梦星星会话已启动，接下来会通过自然语言对话澄清需求并准备三套方案。";
export const BOOTSTRAP_BUSY_NOTICE =
  "正在执行能力安装/校验、星梦梦语义验收和初始化协作包落盘，真实请求可能需要几十秒；窗口没有卡死，请等待动效结束。";

export type BootstrapProgressStatus = "pending" | "running" | "done";

export interface CapabilitySummary {
  id?: string;
  label: string;
  selected?: boolean;
  required?: boolean;
  locked?: boolean;
  detail?: string;
}

export interface BootstrapProgressItem {
  label: string;
  detail: string;
  status: BootstrapProgressStatus;
  motion: boolean;
}

export interface ConversationStatusView {
  kind: "busy" | "error" | "idle";
  message?: string;
}

export interface SolutionDialogSessionLike {
  stage?: string;
  solutions?: Array<{ id: string }>;
  toolCalls?: Array<{ toolName?: string; status?: string }>;
  finished?: boolean;
}

export interface ProviderCardLike {
  providerId: string;
  blockingErrors?: string[];
  authMode?: string;
  configDetected?: boolean;
  defaultBaseUrl?: string | null;
}

export interface TargetClientOption {
  id: "codex" | "claude-code";
  label: string;
  entryName: string;
  title: string;
  description: string;
  disabled: boolean;
  disabledReason?: string;
}

export function shouldRenderProviderCard(provider: Pick<ProviderCardLike, "providerId">): boolean {
  return Boolean(provider.providerId);
}

export function shouldDisableProviderCard(provider: ProviderCardLike): boolean {
  return Boolean(provider.blockingErrors?.includes("not_in_first_wave"));
}

export function getProviderAvailabilityText(provider: ProviderCardLike): string {
  if (provider.blockingErrors?.includes("not_in_first_wave")) {
    return WORK_IN_PROGRESS_TEXT;
  }
  if (provider.providerId === "codex" && provider.authMode === "unsupported_official_login") {
    return "官方授权暂不支持";
  }
  if (provider.configDetected) {
    return "已发现本地配置";
  }
  if (provider.providerId === "custom") {
    return "需要远程地址与认证";
  }
  return provider.defaultBaseUrl ? "标准 API 渠道" : "等待配置";
}

export function getTargetClientOptions(): TargetClientOption[] {
  return [
    {
      id: "codex",
      label: "Codex",
      entryName: "AGENTS.md",
      title: "AGENTS.md 工作流",
      description: "生成 Codex 原生入口、.codex 调度文件和 .agents skills。",
      disabled: false,
    },
    {
      id: "claude-code",
      label: "Claude Code",
      entryName: "CLAUDE.md",
      title: "CLAUDE.md 工作流",
      description: "Claude Code 原生入口、.claude agents 与项目设置将在 v2.0 开放。",
      disabled: true,
      disabledReason: WORK_IN_PROGRESS_TEXT,
    },
  ];
}

export function buildAgentStartLog(): string {
  return AGENT_START_LOG;
}

export function getConversationTitle(): string {
  return CONVERSATION_STAGE_TITLE;
}

export function getSendButtonLabel(isBusy: boolean): string {
  return isBusy ? STAR_THINKING_LABEL : SEND_TO_STAR_LABEL;
}

export function getSendButtonViewLabel(input: {
  conversationBusy: boolean;
  recoverableError?: string;
}): string {
  if (input.conversationBusy) {
    return STAR_THINKING_LABEL;
  }
  return input.recoverableError ? `重试${SEND_TO_STAR_LABEL}` : SEND_TO_STAR_LABEL;
}

export function getConversationStatusView(input: {
  busyNotice?: string | null;
  recoverableError?: string;
}): ConversationStatusView {
  if (input.busyNotice) {
    return { kind: "busy", message: input.busyNotice };
  }
  if (input.recoverableError) {
    return { kind: "error", message: input.recoverableError };
  }
  return { kind: "idle" };
}

export function getWorkspacePathChipText(workspacePath?: string | null): string | null {
  const trimmed = workspacePath?.trim();
  return trimmed ? trimmed : null;
}

export function shouldRenderDiagnosticLogs(diagnosticLogs: string[]): boolean {
  return diagnosticLogs.some((log) => log.trim().length > 0);
}

export function getBootstrapButtonLabel(isBusy: boolean): string {
  return isBusy ? STAR_BOOTSTRAPPING_LABEL : "确认方案并开始初始化";
}

export function getBootstrapBusyNotice(): string {
  return BOOTSTRAP_BUSY_NOTICE;
}

export function getThinkingMascotAriaLabel(context: "conversation" | "bootstrap"): string {
  return context === "bootstrap"
    ? "梦星星和星梦梦正在处理初始化协作包"
    : "梦星星正在思考";
}

export function createBootstrapProgressItems(
  capabilities: CapabilitySummary[],
  targetClient: "codex" | "claude-code",
): BootstrapProgressItem[] {
  const targetClientLabel = targetClient === "claude-code" ? "Claude Code" : "Codex";
  const capabilityText = capabilities.map((capability) => capability.label).join("、");

  return [
    {
      label: "记录能力选择",
      detail: capabilityText,
      status: "running",
      motion: true,
    },
    {
      label: "准备目标软件路径",
      detail:
        targetClient === "claude-code"
          ? "CLAUDE.md、.claude/agents、.claude/settings.json"
          : "AGENTS.md、.codex/agents、.agents/skills",
      status: "pending",
      motion: false,
    },
    {
      label: "执行能力安装/校验策略",
      detail: `按 ${targetClientLabel} 写入能力清单，失败项会记录 fallback/blocked 状态`,
      status: "pending",
      motion: false,
    },
    {
      label: "星梦梦语义验收",
      detail: "复核梦星星方案、角色取舍、目标软件和最终产物语义，不通过则阻断成功",
      status: "pending",
      motion: false,
    },
    {
      label: "生成初始化协作包并 postcheck",
      detail: "写入 .commonhe/session、docs 真源文档，并阻断未通过结果",
      status: "pending",
      motion: false,
    },
  ];
}

export function advanceBootstrapProgress(items: BootstrapProgressItem[]): BootstrapProgressItem[] {
  const runningIndex = items.findIndex((item) => item.status === "running");
  if (runningIndex < 0) {
    return items;
  }

  return items.map((item, index) => {
    if (index <= runningIndex) {
      return { ...item, status: "done", motion: false };
    }
    if (index === runningIndex + 1) {
      return { ...item, status: "running", motion: true };
    }
    return { ...item, motion: false };
  });
}

export function completeBootstrapProgress(items: BootstrapProgressItem[]): BootstrapProgressItem[] {
  return items.map((item) => ({ ...item, status: "done", motion: false }));
}

export function getBootstrapProgressIcon(status: BootstrapProgressStatus) {
  if (status === "running") {
    return "thinking";
  }

  if (status === "done") {
    return "✓";
  }

  return "·";
}

export function shouldShowSolutionDialog(
  session: SolutionDialogSessionLike | null | undefined,
): boolean {
  return Boolean(
    session &&
      !session.finished &&
      session.stage !== "bootstrapping" &&
      ((session.toolCalls ?? []).some(
        (toolCall) =>
          toolCall.toolName === "open_solution_selector" && toolCall.status === "requested",
      ) ||
        (session.stage === "solutions_ready" && (session.solutions?.length ?? 0) > 0)),
  );
}
