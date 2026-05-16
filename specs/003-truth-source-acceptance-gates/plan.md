# Implementation Plan: 真源规则自动验收门禁

**Branch**: `[003-truth-source-acceptance-gates]` | **Date**: 2026-05-10 | **Spec**: [spec.md](./spec.md)

## Summary

本期把 `docs/CommonHE开发目标.md` 的硬规则转成机器可读 manifest 与统一断言脚本，并接入 PowerShell orchestrator、桌面主流程测试、资源包 smoke 与 bootstrap postcheck。

## Technical Context

**Language/Version**: PowerShell、TypeScript、Rust  
**Primary Dependencies**: 现有 Tauri desktop、PowerShell orchestrator/bootstrap/postcheck  
**Storage**: `config/commonhe-truth-source-gates.json`、`.commonhe/session/status.json`、postcheck summary  
**Testing**: PowerShell tests、`npm test`、`npm run build`、`cargo test`

## Implementation Strategy

1. 建立 `config/commonhe-truth-source-gates.json`，记录仓库级与目标产物级真源规则。
2. 新增 `tools/assert-commonhe-truth-source.ps1`，同时支持 `-RepoRoot` 与 `-GeneratedRoot -TargetClient`。
3. 先在 PowerShell 测试中增加红灯断言，再实现脚本和 postcheck 集成。
4. 将断言结果写入 postcheck summary 与 `status.json`，失败时阻断成功收口。
5. 更新资源同步 smoke，确保 manifest、断言脚本、`.specify`、`agency-agents-zh` 都进入桌面资源包。

## Risk Controls

- 不用断言脚本替代现有细粒度 postcheck，而是作为统一真源门禁叠加。
- 所有测试 fixture 写入 `tmp/` 或系统临时目录。
- 目标产物检查只允许 Codex 或 Claude Code 当前一期目标。
- 如果规则无法验证，默认按失败处理，不能伪装成功。
