# Task State Machine

## 目标

统一任务在 HE 中的状态迁移，避免"感觉完成了"。

## 状态定义

- `proposed` — 需求已提出，未经批准
- `approved` — 用户已批准启动
- `contract_negotiated` — Sprint Contract 双方确认（可选）
- `in_progress` — 实现进行中
- `evaluating` — 迭代评估中，可循环回 `in_progress`（可选）
- `implementation_done` — 实现者申报完成
- `review_failed` — 审查未通过
- `qa_failed` — 测试未通过
- `accepted` — 通过验收
- `archived` — 已归档

## 状态迁移规则

```
proposed --> approved --> contract_negotiated --> in_progress <--> evaluating
                |                                                      |
                +-- (无需 contract 时直接) --> in_progress             |
                                                  |                    v (所有维度达标)
                                                  +-- (无评估者) --> implementation_done
                                                                      |     |     |
                                                                      v     v     v
                                                            review_failed qa_failed accepted --> archived
                                                                  |         |
                                                                  +---------+--> in_progress
```

## 使用原则

- 未经批准不得从 `proposed` 进入实施
- 开发完成不等于最终完成
- review 和 qa 都可以打回
- 只有通过验收后才能进入 `accepted`

## 可选状态说明

- `contract_negotiated`：当任务不需要 Sprint Contract（见 `SPRINT-CONTRACT.md` 跳过条件）时，可从 `approved` 直接进入 `in_progress`
- `evaluating`：当未启用评估者协议（见 `EVALUATOR-PROTOCOL.md` 触发条件）时，可从 `in_progress` 直接进入 `implementation_done`
- `evaluating` -> `in_progress` 的迭代循环不应超过 3 次；超过阈值后必须升级为人工决策
