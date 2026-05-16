# Desktop Main Flow Contract

## `create_agent_session`

### Input

- `provider`
- `model`
- `apiKey`
- `baseUrl`
- `workspacePath`

### Output

- `sessionId`
- `stage`
- `messages`
- `understandingSummary`
- `solutions`
- `toolCalls`
- `finished`
- `readiness` 或等价 readiness 字段

## `send_agent_message`

### Input

- `sessionId`
- `message`

### Output Guarantees

- 调用期间前端必须可显示 loading 状态
- 返回结果必须包含新的 messages、readiness 和 stage
- 信息不完整时 `stage` 必须保持在对话态

## `choose_agent_solution`

### Input

- `sessionId`
- `solutionId`

### Output Guarantees

- 必须触发真实 bootstrap/postcheck 链路或其桥接
- 必须返回 `DesktopBootstrapResult` 所需的状态信息
- 若工作区未生成产物或 postcheck 失败，返回结果必须阻断成功收口

## Test Harness Contract

统一测试入口必须：

- 只在 `tmp/` 下创建夹具和临时工作区
- 能验证至少一个成功路径和一个失败阻断路径
- 输出用户可读的通过/失败摘要
