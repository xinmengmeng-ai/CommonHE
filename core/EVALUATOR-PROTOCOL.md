# Evaluator Protocol

## 目标

定义迭代评估（Evaluation）的通用行为规范。评估者在实现阶段提供结构化反馈循环，使实现质量在最终门禁前持续提升。

## 设计原理

1. 自我评估不可靠：Agent 评估自己的产出时倾向于自我肯定，即使质量平庸
2. 分离生成与评估：由独立评估者给出反馈比让实现者自我批评更可调校
3. 迭代反馈优于一次性门禁：多轮评分反馈驱动逐步改进

## 架构约束与诚实定位

Anthropic 文章中的 Generator 和 Evaluator 是两个完全独立的 agent 进程，各有独立上下文窗口，通过文件通信。

大多数 IDE 内 agent 环境（Codex/Cursor/类似工具）的现实约束：

- 单线程对话中的角色切换 ≠ 真正的进程隔离
- 同一对话中的"评估者"已看到"实现者"的推理，存在认知偏见
- LLM 有自我一致性倾向，同一模型既实现又评估时倾向为自己辩护

**本协议的定位：结构化检查纪律 + 客观锚点 + 尽可能的上下文隔离。不是对抗性制衡。**

## 三级评估者可信度模型

| 级别 | 评估手段 | 可信度 | 原因 |
|------|---------|--------|------|
| **L1** | 自动化测试 + CI 门禁 | 最高 | 零偏见，测试要么过要么不过 |
| **L2** | 浏览器/API 交互验证 | 高 | 产出物是否工作是客观事实 |
| **L3** | LLM 结构化评分 | 中 | 受限于认知偏见，但有结构化锚点 |

**原则：L1 和 L2 是主要评估手段，L3 是补充。不可仅依赖 L3。**

## 评估者定位

评估者不是一个新的员工角色，而是一种协议/行为模式，由 `@reviewer` 或 `@qa` 在迭代评估阶段采用。

| 属性 | 评估者（Evaluator） | 最终门禁（Reviewer/QA Gate） |
|------|---------------------|---------------------------|
| 介入时机 | 实现过程中（`in_progress` <-> `evaluating` 循环） | 实现完成后（`implementation_done` 之后） |
| 交互方式 | 主动与运行中的产出物交互 | 可以只审查代码和测试结果 |
| 反馈形式 | 按维度评分 + 改进建议 | 通过/打回 + 失败标签 |
| 容忍度 | 允许部分维度暂时不达标 | 硬性门禁 |

## 触发条件

必须启用：
- `risk_gate = high`
- 涉及 UI/UX 且有主观质量维度
- 任务范围跨越 3 个以上文件或模块

建议启用：
- `risk_gate = medium`
- 涉及用户可见行为变更
- 新功能首次实现

可跳过：
- `risk_gate = low` 且改动局部明确
- 纯文档或配置变更

## 上下文隔离策略

### 高风险：独立 subagent（真正隔离）

若宿主环境支持独立子进程 agent（如 Cursor Task tool 的 generalPurpose），高风险评估应在独立上下文中执行。独立 subagent 没有看到实现过程的对话历史，具备更真实的判断独立性。

### 中风险：同线程但强制结构化

先运行 L1（自动化测试）和 L2（浏览器/API 验证）收集客观证据，再基于证据执行 L3 评分。

### 低风险：仅 L1 + L2

跳过 L3 评分，仅依赖自动化测试和命令验证。

## 评估执行顺序（强制）

每轮评估必须按以下顺序，不可跳步：

1. **先跑测试**：执行自动化测试，记录结果（L1）
2. **再交互验证**：用浏览器或 API 工具与产出物交互（L2）
3. **最后打分**：基于客观证据按维度评分（L3）

禁止在没有 L1/L2 证据的情况下直接进行 L3 评分。

## 反宽容校准（Anti-Leniency）

1. 先找问题，再说优点
2. 禁止模糊肯定（"整体不错"、"基本可用"）
3. 默认假设存在未发现的缺陷
4. 不受实现者自报完成状态影响
5. 逐条核对 Sprint Contract 验收标准
6. L1/L2 结果与 L3 印象冲突时，以 L1/L2 为准

## 评分输出格式

```yaml
evaluation_round: <轮次>
sprint_contract_ref: "<contract 文件路径>"
isolation_level: "independent_subagent | same_thread_structured | l1_l2_only"
overall_verdict: pass | fail | conditional_pass

l1_evidence:
  tests_run: "<命令>"
  tests_passed: <数量>
  tests_failed: <数量>

l2_evidence:
  browser_checks:
    - action: "<操作>"
      result: "pass | fail"
  api_checks:
    - endpoint: "<路径>"
      expected: "<预期>"
      actual: "<实际>"

dimensions:
  - name: "<维度名>"
    score: <1-10>
    threshold: <阈值>
    verdict: pass | fail
    evidence: "<基于 L1/L2 的证据>"
    issues:
      - severity: high | medium | low
        description: "<问题描述>"
        suggestion: "<改进建议>"

improvement_priorities:
  - "<最高优先改进项>"

next_action: iterate | proceed_to_gate
```

## 迭代策略

- `next_action = iterate` 时，实现者针对 `improvement_priorities` 定向改进
- 连续 3 轮同一维度无改善 -> 升级为人工决策
- 所有维度达标且无 high 级别 issue -> `next_action = proceed_to_gate`

## 评估者与门禁的衔接

评估者通过后任务进入 `implementation_done`，交由正式门禁。
评估者通过不等于门禁通过，两道关卡独立运行。

## 评估记录持久化

每轮评估写入文件，用于跨 context reset 的状态保持和验收证据。

## 进阶：外部 Harness 系统

当项目复杂度超出 IDE 内 subagent 能力边界时，可考虑搭建外部 Python Harness 系统（独立进程级 Planner-Builder-Evaluator 架构），实现真正的进程隔离、独立上下文窗口和程序化的上下文管理。此为后续演进方向。
