# 主控调度指南

本文件服务于主控管理员：用于调度执行代理、控制边界、执行验收门禁。

## 1. 主控原则

- 主控是管理者，不是默认执行者
- 执行代理负责实现，主控负责监管与验收
- 所有结论都必须有证据支撑

## 2. 默认文档入口

主控与执行代理协作时默认先读：

1. `docs/project_context.md`
2. `docs/architecture/01-项目架构设计书.md`
3. `docs/roadmap/01-实施路线图.md`

如项目存在专项真源，应在此处追加。

若项目存在标记为 `must-read` 的外部参考源，主控在派工前必须先完成阅读，并要求执行代理在相关任务开始前一并阅读。

主控在派工前还必须确认以下能力门禁为绿色：

1. `docs/skills/required-capabilities.md`
2. `{{client_skill_path}}`

若能力门禁未通过，当前线程只能修复依赖，不得派工。

初始化结束后的第一轮实施，主控默认先读：

1. `docs/workflow/implementation-kickoff.md`
2. `docs/workflow/first-sprint-contract.md`
3. `docs/workflow/first-task-pack.md`

若未先读 kickoff docs，不建议直接开始拆任务。

## 3. 角色与手册映射

{{roles_and_manuals}}

## 4. 调度触发

{{dispatch_triggers}}

{{domain_workflow_section}}

对于 `landing-page`、`solution-site`、`showcase-site` 等展示型项目：
- 当前阶段边界优先于长期演进设想
- 不得自动扩展为平台工程
- 不得默认先拆 backend / integration
- 应优先推进前台交付、内容落地、结构确认与素材补齐

若 `.commonhe/session/status.json` 显示 `precheck_failed = true`：
- 当前线程只允许修复 `superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit`
- 不得进入 {{blocked_workflow_stages}}

## 5. 监管与核验

主控必须组织以下核验方式：

### 5.1 迭代评估（实现过程中，按需启用）

当任务的 `risk_gate` 为 medium 或 high 时，建议启用评估者协议：
- 启用前先读 `docs/workflow/evaluator-protocol.md`
- 评分时以 `docs/workflow/grading-criteria.md` 为准
- 需要 Contract 时，先基于 `docs/workflow/sprint-contract-template.md` 建立任务级 Contract
- 评估者在实现过程中按 Sprint Contract 提供结构化反馈
- 按评分标准逐维度评分，驱动迭代改进
- 评估者必须主动与产出物交互，不仅审查代码

### 5.2 最终门禁（实现完成后）

{{final_gate_items}}

迭代评估通过不替代最终门禁。

## 6. 变更检测策略

- 中高风险任务必须纳入影响范围分析
- 提交前必须确认变更范围与回归范围一致
- 执行代理不得自行宣称"影响范围清楚"，必须等待核验摘要

## 7. 交接清单

每次派工至少包含：

- 范围（做什么、不做什么）
- 归属文件
- 输入依赖
- 完成定义
- 输出证据
- Sprint Contract 引用（如适用）

## 8. 上下文管理

长时间运行的任务应遵守上下文管理规范：
- 识别上下文退化信号
- 在需要时执行上下文重置并产出交接产物
- 确保关键状态以文件形式持久化
