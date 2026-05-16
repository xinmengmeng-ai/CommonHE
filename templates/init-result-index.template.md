# {{project_name}} 初始化结果索引

{{package_intro}}

{{handoff_summary}}

## 当前状态

- 初始化已完成
- {{postcheck_status}}
- 当前目录已经生成面向 `{{project_name}}` 的初始化协作包
- 后续接管软件：`{{target_client_name}}`
- 若需让新协议生效，请在 `{{target_client_name}}` 中新开会话或重启会话

## 关键入口

- 软件入口：`{{client_entry_file}}`
- 主控/配置入口：`{{client_coordinator_path}}`
- 初始化会话：`.commonhe/session`

## 真源文档

- `docs/project_context.md`
- `docs/architecture/01-项目架构设计书.md`
- `docs/roadmap/01-实施路线图.md`
- `docs/skills/required-capabilities.md`

## 新线程必读

- `docs/project_context.md`
- `docs/roadmap/01-实施路线图.md`
- `docs/workflow/current-stage-user-checklist.md`
- `docs/workflow/acceptance-gates.md`
- 若存在 `must-read` 外部参考源，进入新线程前必须先读完

## 实施启动入口

- `docs/workflow/implementation-kickoff.md`
- `docs/workflow/first-sprint-contract.md`
- `docs/workflow/first-task-pack.md`
- 新线程默认先读这 3 份文档，再开始第一轮实施拆解

## 进阶 workflow（按需启用）

- `docs/workflow/evaluator-protocol.md`
- `docs/workflow/grading-criteria.md`
- `docs/workflow/sprint-contract-template.md`
- `docs/workflow/implementation-kickoff.md`
- `docs/workflow/first-sprint-contract.md`
- `docs/workflow/first-task-pack.md`
- 当任务 `risk_gate >= medium`、启用 evaluator 流程，或需要 Sprint Contract 时，进入实施前先读这些文档

## 当前阶段交付目标

{{current_phase_goal}}

## 当前阶段不要做什么

{{deferred_capabilities}}

## 收口提醒

{{closure_summary}}

请保留以下路径，不要误删：

{{safe_retained_paths}}

## 协作文档

- `docs/workflow/current-stage-user-checklist.md`
- `docs/workflow/archive-policy.md`
- `docs/workflow/acceptance-gates.md`
- `docs/workflow/evaluator-protocol.md`
- `docs/workflow/grading-criteria.md`
- `docs/workflow/sprint-contract-template.md`

## 执行代理与手册

- 执行代理协议：`{{client_agent_path}}`
- 执行代理手册：`docs/agents/*-handbook.md`
- 运行时能力门禁：`{{client_skill_path}}`

## 当前边界

- 当前初始化包只生成初始化协作包
- 当前不生成业务项目成品、业务代码或业务项目脚手架
- 当前默认覆盖生成，但应在执行前明确确认
- 当前初始化线程到此收口，不继续展开业务实现
