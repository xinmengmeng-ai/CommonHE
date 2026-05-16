# Context Management

## 目标

定义长时间运行的 AI 协作任务中，如何管理上下文窗口、何时执行上下文重置、如何构建跨会话交接产物，以及如何识别上下文退化。

## 设计原理

1. **上下文焦虑**：模型接近自认为的上下文极限时会提前收尾、降低质量
2. **上下文重置优于压缩**（长任务）：清空上下文并启动新 agent，配合结构化交接产物传递状态，消除焦虑并保持质量
3. **代价**：交接产物必须有足够的状态信息让新 agent 干净接手

## 策略选择

| 属性 | 压缩（Compaction） | 上下文重置（Context Reset） |
|------|--------------------|-----------------------------|
| 机制 | 摘要化较早对话，保留同一 agent | 终止当前 agent，启动新 agent，文件传递状态 |
| 连续性 | 高 | 低（依赖交接产物质量） |
| 上下文焦虑 | 可能持续 | 彻底消除 |
| 适用场景 | 中等长度、状态简单 | 长时间、状态复杂、质量退化 |

### 默认使用压缩
- 任务预计在单次上下文窗口内完成
- 状态简单（少于 5 个文件）
- 当前 agent 表现稳定

### 必须执行上下文重置
- 单次运行超过 2 小时或 40 轮对话
- Agent 重复已解决问题、遗忘关键要求、提前收尾
- 已修改超过 15 个文件或涉及 3 个以上模块
- 主控判断当前 agent 已无法有效继续

### 禁止频繁重置
- 不应在 10 轮对话内重置（除非明确的质量崩溃）
- 频繁重置说明任务拆分粒度不够

## 交接产物规格（Handoff Artifact）

```yaml
handoff:
  task_id: "<任务ID>"
  created_at: "<时间戳>"
  reason: "context_anxiety | quality_degradation | scope_exceeded | manual_trigger"

  current_state:
    task_status: "in_progress | evaluating"
    completed_items:
      - "<已完成项及证据>"
    remaining_items:
      - "<未完成项>"
    known_issues:
      - "<已知问题>"

  artifacts:
    sprint_contract: "<contract 文件路径>"
    latest_evaluation: "<评估文件路径>"
    files_touched:
      - "<文件路径>"
    test_results: "<测试结果摘要>"

  context_for_next_agent:
    key_decisions:
      - "<关键决策及原因>"
    gotchas:
      - "<需注意的陷阱>"
    next_steps:
      - "<建议的下一步>"
```

### 质量要求
- 必须自包含：新 agent 不需要阅读前一 agent 对话历史
- 文件路径使用项目根目录相对路径
- `completed_items` 必须有可验证证据
- `known_issues` 必须有复现步骤

## 新 Agent 启动协议

新 agent 启动后，按以下顺序读取：
1. 项目真源文档
2. 当前任务的 Sprint Contract
3. 最近的交接产物
4. 最近的评估记录（如有）
5. 项目主控协议

新 agent 必须先向主控确认对当前状态的理解，不得直接开始实现。

## 反模式检测

| 信号 | 严重度 | 建议动作 |
|------|--------|---------|
| 开始"总结到目前为止的工作" | 高 | 准备重置 |
| 重复修复已修过的 bug | 高 | 立即重置 |
| 遗忘 Contract 中的部分要求 | 高 | 重新注入 contract，无效则重置 |
| 输出格式退化 | 中 | 提醒一次，无效则重置 |
| "让我确认一下"频率增加 | 中 | 关注，可能需要重置 |

## 会话状态持久化清单

以下产物必须以文件形式存在，确保跨 context reset 存活：

| 产物 | 创建时机 |
|------|---------|
| Sprint Contract | 实现开始前 |
| 评估记录 | 每轮评估后 |
| 交接产物 | 上下文重置时 |
| 任务决策日志 | 有重要技术决策时 |
