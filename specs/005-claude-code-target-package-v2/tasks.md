# Tasks: v2.0 Claude Code 目标协作包

## TDD

- [ ] 构造 Claude 目标包污染夹具：`CLAUDE.md` 引用 `AGENTS.md` 或 `.codex` 时 truth-source gate 必须失败。
- [ ] release-package 测试新增 Claude Code 目标包生成与 gate 校验。
- [ ] 生成器测试覆盖 `target_client=claude-code` 时 values、session、Speckit init-options、manifest 全部切换到 Claude 口径。

## Implementation

- [ ] 抽象 target-client copy/entry mapping，禁止模板继续硬编码 Codex 文案。
- [ ] 修复 `CLAUDE.md`、docs、workflow、session handoff 中的目标软件文案变量。
- [ ] 修复 `.specify/init-options.json`、`.specify/integration.json` 与 manifest 生成。
- [ ] 增强 `tools/assert-commonhe-truth-source.ps1` 的 Claude 目标包语义污染门禁。

## Verification

- [ ] Codex 目标包 v1.0 回归通过。
- [ ] Claude Code 目标包 v2.0 新门禁通过。
- [ ] `npm test`
- [ ] `cargo test --lib -- --nocapture`
- [ ] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/common-he-init-orchestrator.tests.ps1`
- [ ] `powershell -NoProfile -ExecutionPolicy Bypass -File tests/release-package.tests.ps1 -ReleaseZipPath <zip>`
