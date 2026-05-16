# {{project_name}} 迭代评估协议

## 目标

定义当前项目在实施阶段如何启用 evaluator 流程，并把迭代评估与最终门禁区分开。

## 启用前必须先读

1. `docs/workflow/grading-criteria.md`
2. `docs/workflow/sprint-contract-template.md`
3. `docs/workflow/acceptance-gates.md`

## 评估者定位

- 评估者不是新的常驻角色，而是由 `reviewer` 或 `qa` 在迭代评估阶段采用的一种工作模式
- 评估者负责在最终门禁前提供结构化反馈，不代替最终验收

## 何时启用

- `risk_gate = high` 时必须启用
- `risk_gate = medium` 时建议启用
- 涉及用户可见行为、跨模块改动或主观质量维度时优先启用

## 执行顺序

1. 先建立或确认 Sprint Contract
2. 先跑自动化测试与命令验证
3. 再做浏览器 / API / 运行态交互验证
4. 最后按评分标准给出结构化评分

## 输出要求

- 必须明确 `pass / fail / conditional_pass`
- 必须附带证据
- 必须给出下一轮最高优先改进项
- 不得只写模糊好评

## 与最终门禁的关系

- evaluator 通过不等于最终门禁通过
- 最终门禁仍需结合 `docs/workflow/acceptance-gates.md` 独立执行

## 记录要求

- Sprint Contract、评估记录与关键验证证据应持久化到项目真源
- 长任务应确保这些记录可跨线程复用
