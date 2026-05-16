# Tasks: 双 Agent 语义验收闭环

## TDD

- [x] 梦星星 solutions 缺少角色理由时拒绝进入方案选择。
- [x] 星梦梦语义验收失败时阻断 bootstrap。
- [x] 桌面 UI 显示语义验收状态。
- [x] PowerShell 流程测试覆盖语义审计文件。
- [x] 生成包成功路径覆盖选中方案一致性、五项 selected 能力、绿色能力门禁、旧外显命名与控制字符清洗。
- [x] 生成包接手开发路径覆盖重复编号、未启用角色引用、陈旧初始化清单、Sprint Contract 实例/模板混用。
- [x] 生成包质检覆盖半实例化模板腔、成功包 current-stage 初始化残留，并要求真实落到 `E:\test\test-shop` 后再跑生成物质检。
- [x] 生成物审查规范明确区分星梦梦语义审查、postcheck、truth-source gate 硬门禁与 sanitizer 禁区。
- [x] 语义修复闭环覆盖 Codex 接管入口计划、业务依赖不等于 selectedCapabilities、QA/reviewer 分工、AI/安全省略职责归属。

## Implementation

- [x] 扩展 Rust `AgentSolution` 与 `AgentSessionSnapshot`。
- [x] 新增星梦梦语义验收审计产物。
- [x] bootstrap 前接入 `final-acceptance.json` 门禁。
- [x] 扩展 TypeScript 类型与 UI 文案。
- [x] 扩展 postcheck/status 对语义字段的检查。
- [x] truth-source gate 要求桌面成功包 `selected_capabilities`、`selected_solution_id` 和 `capability_gate_passed` 真实存在且一致。
- [x] 绿色能力门禁下移除根入口临时补救块，并把验收门禁改为初始化协作包口径。
- [x] truth-source gate 与 postcheck 增加接手开发质量检查，生成器同步修复 handoff 文档、角色路由和合同模板拆分。
- [x] Speckit 合同新增 `contracts/generated-package-review.md`，把漏检问题固化为生成物审查规范。
- [x] Speckit 合同明确半实例化模板腔必须阻断，星梦梦 final review 证据必须包含 current-stage、first-sprint-contract、first-task-pack。
- [x] 语义审查 sanitizer 与梦星星 repair 规范区分工作流能力和业务运行依赖，并把修复轮次失败策略改为达到上限后才阻断。

## Verification

- [x] `cargo test --lib` in `apps/desktop/src-tauri`
- [x] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/common-he-init-orchestrator.tests.ps1`
- [x] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/desktop-main-flow.tests.ps1`
- [x] `npm test` in `apps/desktop`
- [x] `npm run build` in `apps/desktop`
