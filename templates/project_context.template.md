# {{project_name}} 项目上下文

{{package_intro}}

{{handoff_summary}}

## 1. 项目概述

- 项目名称：`{{project_name}}`
- 项目类型：`{{project_type}}`
- 初始化包类型：`初始化协作包`
- 后续接管软件：`{{target_client_name}}`
- 当前阶段：`{{current_phase}}`
- 核心目标：`{{core_goal}}`

## 2. 当前目标

{{current_goals}}

## 3. 当前范围

### 在当前范围内

{{in_scope_items}}

### 不在当前范围内

{{out_of_scope_items}}

## 4. 真源规则

当前项目默认真源文档为：

1. `docs/project_context.md`
2. `docs/architecture/01-项目架构设计书.md`
3. `docs/roadmap/01-实施路线图.md`

如存在模块级或专项真源，应在此处追加。

## 5. 外部参考源

{{external_references}}

## 6. 当前阶段边界

{{stage_constraints}}

- 当前产物是初始化协作包，不是业务项目成品、业务代码或业务项目脚手架
- 后续实施应在 `{{target_client_name}}` 中通过 `{{client_entry_file}}` 入口继续

## 7. 后续阶段再考虑的能力

{{deferred_capabilities}}

## 8. 协作角色

当前启用角色：

{{recommended_roles_now}}

后续可启用角色：

{{available_roles_later}}

## 9. 验收口径

当前阶段验收至少应覆盖：

{{acceptance_criteria}}

## 10. 当前阶段任务

{{current_phase_tasks}}

## 11. 风险与约束

{{risks_and_constraints}}

## 12. 自动分析信号

{{autodiscovery_signal_summary}}

## 13. 自动分析假设

{{autodiscovery_assumptions}}
