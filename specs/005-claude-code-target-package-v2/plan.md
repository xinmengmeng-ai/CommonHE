# Implementation Plan: v2.0 Claude Code 目标协作包

**Branch**: `[005-claude-code-target-package-v2]` | **Date**: 2026-05-16 | **Status**: Planned for v2.0

## Summary

v2.0 将补齐 `Claude Code` 作为后续接管软件的目标协作包能力。v1.0 正式发版只承诺 `Codex` 目标协作包；Claude Code 当前不阻断 v1.0，但必须在 v2.0 中修复结构、文案、Speckit 配置、truth-source gate 与 release-package 测试的全链路一致性。

## Problem

当前 `target_client=claude-code` 的临时验证显示：

- 结构层面能生成 `CLAUDE.md`、`.claude/settings.json`、`.claude/agents/`、`.claude/skills/required-capabilities.md`
- 但内容层面仍残留 `Codex`、`AGENTS.md`、`.codex`、`Codex 原生入口` 等 Codex-only 语义
- truth-source gate 对 Claude 目标包只检查入口结构，未阻断语义污染
- release-package 测试未把 Claude 目标包作为并列目标包门禁

## v2.0 Scope

- 生成器按 `target_client` 统一归一化用户可见文案、session 审计、values、Speckit init-options 与 manifest。
- Claude Code 目标包只暴露 `CLAUDE.md` 与 `.claude/` 入口。
- Claude Code 目标包不得生成或引用 `AGENTS.md`、`.codex/`、`Codex 原生入口` 或 “在 Codex 中接手”。
- truth-source gate 新增 Claude 语义污染硬门禁。
- release-package 测试同时生成并验证 Codex 与 Claude Code 目标包。

## Out Of Scope For v1.0

- 不在 v1.0 中开放 Claude Code 目标包承诺。
- 不把 Claude Code 作为桌面第一批正式 provider 渠道。
- 不为了 v1.0 手工修补已生成 Claude 测试包。

## Acceptance Gates

- `powershell -NoProfile -ExecutionPolicy Bypass -File tools/assert-commonhe-truth-source.ps1 -GeneratedRoot <claude-package> -TargetClient claude-code -AsJson` 返回 `Passed=true`。
- 全包搜索不得出现 Codex-only 入口语义污染。
- `CLAUDE.md`、`.claude/settings.json`、`.claude/agents/`、`.claude/skills/required-capabilities.md` 均存在且互相一致。
- Codex 目标包现有 v1.0 生成链路和 release-package 测试保持通过。
