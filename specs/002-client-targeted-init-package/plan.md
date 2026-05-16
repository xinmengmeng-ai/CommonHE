# Implementation Plan: 目标软件定向的初始化协作包收口

**Branch**: `[002-client-targeted-init-package]` | **Date**: 2026-05-10 | **Spec**: [spec.md](./spec.md)

## Summary

本期把桌面主流程从“选方案后直接 bootstrap”扩展为“选方案、填项目名、选目标软件、确认必需能力、再生成初始化协作包”。生成器按目标软件输出 Codex 或 Claude Code 原生入口，并保持 `.commonhe/session/` 与 `docs/` 作为 CommonHE 内部真源。

## Technical Context

**Language/Version**: TypeScript + Rust + PowerShell  
**Primary Dependencies**: Tauri 2、React、现有 PowerShell orchestrator/bootstrap/postcheck  
**Storage**: 目标工作区、`.commonhe/session/`、`docs/`、目标软件配置目录  
**Testing**: `npm test`、`cargo test`、PowerShell desktop tests

## Implementation Strategy

1. 扩展桌面 UI 和 Tauri contract：`projectName`、`targetClient`、`selectedCapabilities`。
2. 扩展 Rust agent session：保存项目名、目标软件、能力选择，写入 session seed。
3. 扩展 PowerShell generator/postcheck：按 target client 生成和校验 Codex/Claude Code 入口。
4. 更新模板与真源文档：统一初始化协作包口径。
5. 补自动测试：覆盖 Codex、Claude Code、项目名、能力选择和错误阻断。

## Risk Controls

- 不改变 provider/model 验证前置门禁。
- 不生成业务代码或业务项目脚手架。
- 临时测试工作区仍只允许写入 `tmp/`。
