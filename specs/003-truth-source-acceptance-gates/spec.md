# Feature Spec: CommonHE 真源规则自动验收门禁

## 目标

把 `docs/CommonHE开发目标.md` 中的硬规则固化为可重复运行的自动验收门禁，避免初始化主流程、provider/model、主 Agent、目标软件协作包、Skill/MCP、Speckit 或 agent 团队生成相关改动绕过真源规则。

## 范围

- 新增机器可读规则清单。
- 新增仓库级与目标产物级断言脚本。
- 将断言结果接入 bootstrap postcheck 与 session status。
- 将资源同步、桌面主流程测试和 orchestrator 测试接入同一门禁。

## 核心规则

- 对外产品名必须是 `星星的vibecoding启动器`。
- provider/model 必须是受控选择，默认不得依赖自由输入模型名。
- `DeepSeek` 必须是一等 provider，默认 `deepseek-v4-flash`，可选 `deepseek-v4-pro`，且必须要求 APIKey。
- `custom` 必须校验 URL、协议、模型、APIKey 与连通性。
- 初始化必须由主 Agent 真实对话驱动，并引用 `product-manager.md` 与 `agency-agents-zh/README.md`。
- 三方案必须包含架构、agent 团队、token 预估，并通过内置选择 UI 完成选择。
- 方案选择后必须确认项目名、目标软件、能力选择。
- Codex / Claude Code 入口互斥。
- Codex 产物必须包含 `AGENTS.md`、`.codex/`、`.agents/skills/`。
- Claude Code 产物必须包含 `CLAUDE.md`、`.claude/`。
- 初始化协作包必须包含 `docs/`、`.commonhe/session/`、`.specify/`；`superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit` 五项能力均为必需能力，不再提供取消入口。
- agent 文件必须来自 `agency-agents-zh` 真实 agent 库映射，不得回退成 `frontend.md`、`backend.md`、`qa.md` 等占位文件。
- 生成文档不得暗示业务项目成品、业务代码或业务脚手架已经生成。

## 成功标准

- `tools/assert-commonhe-truth-source.ps1 -RepoRoot <repo>` 可以检查仓库源码、测试覆盖与资源同步规则。
- `tools/assert-commonhe-truth-source.ps1 -GeneratedRoot <target> -TargetClient codex|claude-code` 可以检查目标初始化协作包。
- bootstrap postcheck 输出 `TruthSourceGatePassed` 与 `TruthSourceGateIssues`。
- `status.json` 写入 `truth_source_gate_passed`、`truth_source_gate_failed`、`truth_source_gate_issues`。
- 任一真源规则失败时不得宣布初始化成功。
