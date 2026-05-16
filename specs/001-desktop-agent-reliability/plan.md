# Implementation Plan: 桌面主Agent可靠性修复与测试框架

**Branch**: `[001-desktop-agent-reliability]` | **Date**: 2026-05-07 | **Spec**: [spec.md](./spec.md)
**Input**: Feature specification from `/specs/001-desktop-agent-reliability/spec.md`

## Summary

本轮实现将修复 `星星的vibecoding启动器` 桌面主路径中的四个阻断问题：对话命名错误、发送消息导致界面假死、方案输出时机错误、方案确认后不落盘。技术策略是把“对话可用性”和“初始化落盘”拆成两个明确链路：前者重构为非阻塞的会话执行与信息完整度驱动，后者在方案确认后显式接入现有 orchestrator/bootstrap/postcheck 能力，并为这两条链路增加可重复执行的自动测试框架。

## Technical Context

**Language/Version**: TypeScript (React 19) + Rust 2024 + PowerShell 5/7  
**Primary Dependencies**: Tauri 2、React、reqwest、serde、现有 CommonHE PowerShell orchestrator  
**Storage**: 文件系统工作区、`.commonhe/session/`、`docs/`、`tmp/`  
**Testing**: `node src/appState.test.ts`、`cargo test`、PowerShell 集成测试、桌面 smoke/build 脚本  
**Target Platform**: Windows desktop (portable exe)  
**Project Type**: Tauri desktop app + local initialization orchestrator  
**Performance Goals**: 发送消息后 200ms 内出现加载反馈；远程请求期间窗口保持可响应  
**Constraints**: 不能制造假成功；不能让 `tmp/` 污染正式文档；必须兼容现有 `bootstrap/postcheck` 产物格式  
**Scale/Scope**: 聚焦桌面主流程、会话桥接、初始化落盘链路与自动测试框架，不扩展新 provider 范围

## Constitution Check

`.specify/memory/constitution.md` 当前仍是未实例化模板，无法提供可执行的 MUST 级治理规则。对本 feature 而言，临时以仓库真源约束替代：

1. `docs/CommonHE开发目标.md` 为产品与流程真源。
2. `AGENTS.md` 为后续线程的 LLM 级约束来源。
3. 先修复 flow-blocking bugs，再做 flow optimization。
4. 所有交付必须先经过自动测试框架验证，不能只靠人工最终点测。

结论：在上述真源约束下，本计划可进入 Phase 0/1/2。

## Project Structure

### Documentation (this feature)

```text
specs/001-desktop-agent-reliability/
├── spec.md
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
└── tasks.md
```

### Source Code (repository root)

```text
apps/desktop/
├── src/
│   ├── App.tsx
│   ├── agentFlow.ts
│   ├── appState.ts
│   ├── appState.test.ts
│   └── tauriApi.ts
└── src-tauri/
    └── src/commonhe_bridge/
        ├── agent.rs
        ├── commands.rs
        ├── provider.rs
        └── shell.rs

tests/
├── common-he-init-orchestrator.tests.ps1
├── desktop-smoke.tests.ps1
└── [new desktop main-flow tests]

tmp/
└── [new test fixtures and generated workspaces]

docs/
└── [truth-source and testing framework documentation]
```

**Structure Decision**: 保持现有单仓多层结构，不拆新应用。桌面 UI 改动集中在 `apps/desktop/src/`，Tauri/bridge 改动集中在 `apps/desktop/src-tauri/src/commonhe_bridge/`，自动测试框架集中在 `tests/` 和 `tmp/`，正式说明集中在 `docs/`。

## Phase 0: Research

1. 确认 Tauri 命令导致窗口假死的根因。
   - 决策：将会话请求从阻塞式调用迁移到异步/后台执行路径，并在前端建立显式 loading 状态。
   - 理由：当前 `reqwest::blocking` + 同步 Tauri command 最可能直接占住窗口线程。
   - 备选：仅在前端加 spinner 被否决，因为无法解决窗口未响应根因。

2. 确认方案输出过早的根因。
   - 决策：用 `ConversationReadinessState` 替代固定用户轮次阈值。
   - 理由：产品需要“理解足够 + 用户确认”后才能出方案。
   - 备选：把阈值从 2 改成 5 被否决，因为仍然是假规则。

3. 确认落盘缺失的根因。
   - 决策：方案确认后显式调用 orchestrator bootstrap/postcheck 能力或等价桥接，而不是只结束内存会话。
   - 理由：现有生成能力已经成熟，桌面主路径应优先复用。
   - 备选：单独在桌面端新写一套落盘器被否决，因为会与兼容路径分叉。

4. 确认 Speckit 与测试框架在 Windows 下的约束。
   - 决策：本轮先用 Speckit 目录与模板作为正式计划载体，并补充 Windows 友好的测试入口；将 `bash/sh` 缺口列入后续工具完善范围。
   - 理由：当前机器上 `speckit` CLI 自动脚本链依赖的 `bash` 不可用，但计划与执行约束本身仍可落地。

## Phase 1: Design & Contracts

### Data Model

- `ConversationReadinessState`
- `DesktopSolutionSelection`
- `DesktopBootstrapResult`
- `DesktopTestFixture`

详见 [data-model.md](./data-model.md)。

### Interface Contracts

桌面主路径需要稳定以下接口：

1. `create_agent_session`
2. `send_agent_message`
3. `choose_agent_solution`
4. 新增或扩展的 bootstrap/result contract
5. 自动测试框架入口 contract

本 feature 采用 markdown 合同文档，详见 `contracts/desktop-main-flow.md`。

### Agent Context Update

`AGENTS.md` 中 `<!-- SPECKIT START -->` 与 `<!-- SPECKIT END -->` 之间必须指向本计划文件：

`specs/001-desktop-agent-reliability/plan.md`

## Phase 2: Implementation Strategy

### Story Mapping

- **US1**: UI 文案与发送加载态、非阻塞调用链
- **US2**: 信息完整度判断与用户确认门槛
- **US3**: 方案确认后触发真实 bootstrap/postcheck 落盘
- **US4**: `tmp/` 测试框架、`docs/` 文档约定、AGENTS/Speckit 约束

### Test Strategy

1. 先补纯逻辑测试：命名、状态机、readiness 决策。
2. 再补 Rust 单元测试：会话状态、bootstrap 结果 contract。
3. 再补 PowerShell 集成测试：临时工作区自动生成、postcheck 校验、失败阻断。
4. 最后跑现有 smoke/build 回归。

### Risk Controls

- 所有临时测试工作区只允许创建在 `tmp/`。
- 不允许把正式文档生成到 `tmp/` 之外的测试路径。
- 不允许在未写入目标工作区或未通过 postcheck 时发出成功收口。

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|-----------|------------|-------------------------------------|
| 复用旧 orchestrator 进入桌面主路径 | 避免重新实现一套不一致的生成与 postcheck | 完全重写桌面落盘器会扩大风险并造成双轨结果 |
