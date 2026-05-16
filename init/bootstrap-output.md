# Bootstrap Output

## 目标

定义用户确认方案后，初始化器应该生成什么内容。

## 默认生成物

- `docs/00-初始化结果索引.md`
- `AGENTS.md`
- `.codex/COORDINATOR-SUBAGENTS.md`
- `.codex/agents/*.md`
- `docs/project_context.md`
- `docs/architecture/01-项目架构设计书.md`
- `docs/roadmap/01-实施路线图.md`
- `docs/workflow/current-stage-user-checklist.md`
- `docs/workflow/archive-policy.md`
- `docs/workflow/acceptance-gates.md`
- `.commonhe/session/bootstrap-handoff.md`

## 可按项目追加

- 数据库角色
- DevOps 角色
- 集成角色
- 合规角色
- 特定领域员工与手册
- 外部风格参考源
- 外部内容参考源

## 生成后强制动作

初始化器在落模板后，必须继续完成以下动作：

1. 执行 `postcheck`
2. 校验 AI Agents team 与 `decision.json` 一致
3. 若 `postcheck` 失败，则输出缺失项 / 冗余项摘要，并阻断“初始化成功”
4. 若 `postcheck` 通过，则刷新实施阶段真源文档
5. 若 `postcheck` 通过，则输出成功收口提示，要求用户新开线程或重启 `Codex`
