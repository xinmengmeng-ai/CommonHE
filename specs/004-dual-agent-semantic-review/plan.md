# Implementation Plan: 双 Agent 语义验收闭环

**Branch**: `[004-dual-agent-semantic-review]` | **Date**: 2026-05-11 | **Spec**: [spec.md](./spec.md)

## Summary

本期新增供应商无关的双 Agent 语义验收闭环：梦星星输出结构化方案与角色取舍理由，星梦梦在 bootstrap 前完成语义复核并写入审计产物。语义验收不通过时阻断成功，避免生成产物错误后靠人工修补。

## Technical Context

**Language/Version**: Rust、TypeScript、PowerShell  
**Primary Dependencies**: 现有 Tauri desktop、provider adapter、PowerShell orchestrator/postcheck  
**Storage**: `.commonhe/session/*semantic*` 与 `final-acceptance.json`  
**Testing**: Rust unit tests、PowerShell desktop tests、npm tests、cargo test

## Implementation Strategy

1. 扩展梦星星输出 schema，强制三方案包含角色选择理由与不选理由。
2. 在 Rust 桥接层增加星梦梦语义验收结果、修正记录与最终验收落盘。
3. 在 bootstrap 前检查 `final-acceptance.json`，未通过则不执行目标工作区写入。
4. 前端展示语义验收状态，失败时显示星梦梦阻断项。
5. 保留现有 truth-source gate/postcheck 作为最终硬门禁。

## Risk Controls

- 不引入第三方 Agent SDK。
- 星梦梦不直接改产物，只输出阻断项和修正要求。
- 兼容路径继续保留，但桌面主流程成功必须包含语义验收状态。
- 生成包审计必须闭环：`decision.json`、`meng-xingxing-output.json`、`final-acceptance.json` 中的选中方案必须一致。
- 成功包的 `selected_capabilities` 必须包含五项必选能力，`status.capability_gate_passed=true`；`required_capabilities` 不能替代用户确认后的 selected 记录。
- 绿色能力门禁不得生成“临时补救命令”，用户可见文档不得外显旧产品名、控制字符或“业务实现/review/QA 已完成”类假成功口径。
- 接手开发包必须额外通过 handoff 质量门禁：入口文档不得重复编号；业务工作流不得引用未启用角色；`current-stage-user-checklist.md` 不得要求新线程重做初始化/postcheck/bootstrap/初始化落盘；`first-sprint-contract.md` 必须是当前项目第一轮实施合同实例，不得与通用 `sprint-contract-template.md` 完全相同，也不得残留“模板/复制/待填写”等半实例化模板腔。
- 生成物审查边界必须按 `contracts/generated-package-review.md` 执行：星梦梦做语义挑刺，postcheck/truth-source gate 做确定性阻断，sanitizer 不得吞掉硬门禁问题。
