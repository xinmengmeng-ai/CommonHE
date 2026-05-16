# Sprint Contract

## 目标

定义 Sprint Contract（冲刺契约）的通用协商、编写与使用规范。Sprint Contract 是实现者与评估者在工作开始前就"做什么"和"怎么算完成"达成的书面协议。

## 为什么需要 Sprint Contract

| 问题 | Sprint Contract 如何解决 |
|------|--------------------------|
| 高层需求太抽象 | 在开工前拆解为具体、可测试的验收标准 |
| "完成"标准模糊 | 书面约定可测试的完成定义 |
| 评估者不知道该测什么 | Contract 即是评估者的测试清单 |
| 范围蠕变 | 明确约定 in-scope 和 out-of-scope |
| 上下文丢失 | 文件持久化，跨 context reset 存活 |

## 触发条件

必须创建：
- 启用了评估者协议的任务
- `risk_gate = medium` 或 `high`
- 涉及 3 个以上可独立验证的功能点

建议创建：
- 新功能首次实现
- 跨多个模块的变更

可跳过：
- `risk_gate = low` 且改动明确
- Bug 修复（已有明确复现步骤）
- 纯重构（无行为变更）

## 协商流程

1. 主控派发任务（含需求摘要）
2. 实现者起草 Contract 草案
3. 评估者审查（关注"标准是否可测试"、"范围是否合理"）
4. 迭代直到双方一致（不超过 2 轮，超出则升级主控裁决）
5. Contract 定稿，进入实现

## Contract 模板

```markdown
# Sprint Contract: <任务标题>

## 元信息
- task_id: <任务ID>
- implementer: <实现者角色>
- evaluator: <评估者角色>
- risk_gate: low | medium | high
- created: <日期>
- status: draft | agreed | in_progress | completed

## 需求摘要
<1-3 句话>

## 交付物清单
1. <交付物>
2. ...

## 验收标准
| # | 标准描述 | 验证方法 | 关联交付物 |
|---|---------|---------|-----------|
| 1 | <可验证的行为描述> | <如何测试> | <编号> |

## 评分维度与阈值
| 维度 | 阈值 | 权重 |
|-----|------|------|
| 功能完整性 | >= 6 | 高 |
| 代码质量 | >= 6 | 中 |

## 范围约定
### In-scope
- <明确要做的事>

### Out-of-scope
- <明确不做的事>

## 已知风险
- <风险>

## 测试策略
- <覆盖范围>
```

## Contract 持久化

- Contract 必须作为文件持久化
- 一旦双方确认（`status: agreed`），不得单方面修改
- 需要调整时，双方重新协商并记录变更原因

## Contract 与状态机

对应任务状态机中的 `contract_negotiated` 状态。
不需要 Contract 的任务可从 `approved` 直接进入 `in_progress`。

## Contract 与评估

评估者每轮迭代必须逐条核对 contract 验收标准，标记 `met` / `not_met` / `partially_met`。
所有标准 `met` 且所有评分维度达标时，方可 `proceed_to_gate`。
