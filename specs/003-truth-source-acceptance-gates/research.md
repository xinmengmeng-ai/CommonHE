# Research: 真源规则自动验收门禁

## Decision: 统一脚本作为门禁入口

现有 postcheck 已经有目标软件入口、`.specify`、agent 文件和著作质量检查，但规则分散在 orchestrator 内部。新增脚本可以被测试、postcheck、发布 smoke 和人工命令复用。

## Decision: manifest 存规则，不存实现逻辑

`config/commonhe-truth-source-gates.json` 描述规则 ID、来源、严重级别、模式与关键字段；具体路径遍历、文本检查、目标软件互斥等逻辑放在 PowerShell 脚本中，避免 JSON 变成弱表达式语言。

## Decision: 目标产物检查失败时阻断成功收口

真源规则失败属于 flow-blocking bug，因为它会造成 false success。postcheck 的 `Passed` 必须包含 truth-source gate 结果。

## Decision: 仓库级检查偏向结构与测试覆盖

仓库级检查不直接运行 provider live 网络请求，但会确认 DeepSeek/custom/model 选择等规则在源码和测试中有明确结构和断言。
