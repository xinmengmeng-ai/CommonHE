# Feature Specification: 目标软件定向的初始化协作包收口

**Feature Branch**: `[002-client-targeted-init-package]`  
**Created**: 2026-05-10  
**Status**: In Progress  
**Input**: 用户要求本期完成方案选择后的项目名收口、目标软件选择、Skill/MCP 必需能力确认，并统一 CommonHE 只生成初始化协作包的口径。

## User Scenarios & Testing

### User Story 1 - 选择方案后确认协作包项目名 (P1)

作为用户，我希望在选定方案后再填写初始化协作包项目名，这样生成的真源文档不会被工作区文件夹名污染。

**Acceptance**:
- 方案选择后必须出现项目名确认。
- 项目名为空或包含非法路径字符时不得 bootstrap。
- `answers.json`、`decision.json`、`generated-values.json` 必须使用用户确认的项目名。

### User Story 2 - 选择后续使用的软件 (P1)

作为用户，我希望明确选择后续由 Codex 还是 Claude Code 接管，以便生成对应软件原生入口。

**Acceptance**:
- 第一版只提供 `Codex` 和 `Claude Code`。
- Codex 生成 `AGENTS.md` 与 `.codex/` 入口。
- Claude Code 生成 `CLAUDE.md` 与 `.claude/` 入口。

### User Story 3 - 确认必需能力 (P1)

作为用户，我希望在生成前看到必需安装或配置的 Skill/MCP/CLI 能力，并明确知道这些能力当前版本不可取消。

**Acceptance**:
- 默认勾选五项必需能力。
- 用户不能取消 `superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit`。
- 用户选择与安装/回退状态写入 session。
- 能力安装或校验失败时不得伪装成功。

### User Story 4 - 文档口径保持初始化协作包 (P1)

作为维护者，我希望生成文档明确说明这是初始化协作包，不是业务项目成品或业务代码脚手架。

**Acceptance**:
- 初始化结果索引、项目上下文、架构/路线图/workflow 文档不误导为业务成品。
- 收口提示要求在所选软件中新开会话继续实施。

## Functional Requirements

- **FR-001**: 桌面方案选择后 MUST 收集 `projectName`。
- **FR-002**: `projectName` MUST 写入所有 session 与生成值文件。
- **FR-003**: 桌面方案选择后 MUST 收集 `targetClient`，第一期仅 `codex` 与 `claude-code`。
- **FR-004**: Bootstrap MUST 按 `targetClient` 生成对应入口文件。
- **FR-005**: Bootstrap MUST 记录五项 mandatory `selectedCapabilities`。
- **FR-006**: Postcheck MUST 按 `targetClient` 校验对应入口。
- **FR-007**: 文档 MUST 使用“初始化协作包”口径。

## Success Criteria

- **SC-001**: Codex 路径生成 `AGENTS.md`、`.codex/`、`.commonhe/session/`、`docs/` 并通过 postcheck。
- **SC-002**: Claude Code 路径生成 `CLAUDE.md`、`.claude/`、`.commonhe/session/`、`docs/` 并通过 postcheck。
- **SC-003**: 项目名不再从 workspace 文件夹名推断。
- **SC-004**: 生成文档不声称业务项目成品已生成。
