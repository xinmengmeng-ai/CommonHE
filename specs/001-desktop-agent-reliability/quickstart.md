# Quickstart: 桌面主Agent可靠性修复与测试框架

## 1. 运行前端逻辑测试

在 `apps/desktop` 执行：

- `npm test`

## 2. 运行 Rust 单元测试

在 `apps/desktop/src-tauri` 执行：

- `cargo test`

## 3. 运行桌面主流程自动测试框架

在仓库根目录执行：

- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/desktop-main-flow.tests.ps1`

预期：

- 自动在 `tmp/desktop-main-flow/` 下创建测试工作区
- 验证命名、加载态、readiness、方案确认后落盘和 postcheck 行为
- 输出通过/失败状态并自动清理可清理的临时数据

## 4. 运行现有回归

- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/desktop-smoke.tests.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/common-he-init-orchestrator.tests.ps1`
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/build-desktop.ps1 -SkipInstall`

## 5. 运行真实 Provider 冒烟

- `powershell -NoProfile -ExecutionPolicy Bypass -File tests/live-provider-smoke.tests.ps1 -DeepSeekApiKey '<临时测试Key>'`

预期：

- `DeepSeek` 在给定临时测试 key 时完成真实联网冒烟
- `Codex` 在发现本地 `auth.json` / `config.toml` 后完成来源审计
- 若 `Codex` 当前 wire API 仍为未支持的 `responses`，测试摘要会明确标记为 `blocked`
