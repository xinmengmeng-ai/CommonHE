# Data Model: 桌面主Agent可靠性修复与测试框架

## ConversationReadinessState

- `productType`: MCP / Skill / 网站 / 软件 / 其他
- `targetUsers`: 目标用户描述
- `coreProblem`: 要解决的核心问题
- `keyFeatures`: 用户列出的关键功能集合
- `constraints`: 技术、时间、部署或资源约束
- `summaryPresented`: 是否已经由 `星星` 输出总结
- `summaryConfirmed`: 用户是否确认总结准确
- `missingFields`: 当前仍缺失的关键信息列表
- `readyForSolutions`: 是否允许进入三方案阶段

## DesktopSolutionSelection

- `solutionId`: 方案 ID
- `title`: 方案标题
- `selected`: 是否已被用户选中
- `confirmedAt`: 确认时间
- `bootstrapRequested`: 是否已请求初始化落盘
- `bootstrapCompleted`: 是否已完成落盘
- `postcheckPassed`: 是否已通过 postcheck

## DesktopBootstrapResult

- `workspacePath`: 目标工作区
- `generatedFiles`: 生成文件摘要
- `handoffPath`: handoff 文档路径
- `status`: success / failure
- `postcheckSummary`: postcheck 结果摘要
- `userFacingMessage`: 收口给用户看的消息

## DesktopTestFixture

- `fixtureId`: 测试夹具编号
- `workspaceRoot`: `tmp/` 下的临时工作区
- `providerStubMode`: provider 模拟模式
- `conversationScript`: 测试输入脚本
- `expectedStageTransitions`: 预期阶段变化
- `expectedFiles`: 预期生成文件
- `cleanupPolicy`: 清理策略
