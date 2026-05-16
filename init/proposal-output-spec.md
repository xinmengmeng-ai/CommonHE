# Proposal Output Spec

## 目标

约束 `CommonHE/init` 在 proposal 阶段输出的结构，保证不同项目里方案展示足够稳定。

## 标准输出结构

每套方案建议包含以下字段：

1. `name`
2. `positioning`
3. `best_for`
4. `core_stack_suggestion`
5. `time_cost`
6. `deployment_cost`
7. `development_difficulty`
8. `scalability`
9. `risks`
10. `recommendation_reason`

## 用户可见描述要求

- 不堆术语
- 尽量给出“人话版含义”
- 对成本和难度要有倾向性说明

## 结尾要求

proposal 输出最后必须有一个清晰的收口动作：

- 用户确认选哪套方案
- 用户要求混合方案
- 用户要求继续补充信息

## 生成门禁要求

- proposal 阶段只能产出候选方案
- 用户未明确拍板前，不得自动进入 bootstrap
- 只有在用户确认后，才能把结果落为 `decision.json`
