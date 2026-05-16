# Tasks: 真源规则自动验收门禁

## TDD

- [ ] 新增 truth-source gate 红灯测试。
- [ ] 新增资源同步 smoke 断言。
- [ ] 新增桌面主流程目标产物 gate 断言。

## Implementation

- [ ] 新增 `config/commonhe-truth-source-gates.json`。
- [ ] 新增 `tools/assert-commonhe-truth-source.ps1`。
- [ ] 接入 `Invoke-PostBootstrapCheck`。
- [ ] 扩展 `Set-Status` 与 `Show-Status`。
- [ ] 更新 bootstrap handoff 失败项汇总。
- [ ] 更新 `scripts/sync-desktop-resources.ps1` 覆盖新增资源。

## Verification

- [ ] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/common-he-init-orchestrator.tests.ps1`
- [ ] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/desktop-main-flow.tests.ps1`
- [ ] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/desktop-smoke.tests.ps1`
- [ ] `npm test` in `apps/desktop`
- [ ] `npm run build` in `apps/desktop`
- [ ] `cargo test` in `apps/desktop/src-tauri`
