# Research: 双 Agent 语义验收闭环

## 结论

多 Agent 编排可以参考公开方案中的 handoff、supervisor、agent-as-tool 和 review loop，但 `星星的vibecoding启动器` 不能直接绑定任一供应商 SDK。

## 采用策略

- 内部实现轻量编排协议。
- provider 调用继续走现有 adapter。
- 所有关键结果结构化落盘。
- 静态脚本负责硬规则，星梦梦负责语义挑刺。
