# Bootstrap Manifest

## 目标

定义 `CommonHE/init` 在用户确认方案后，应如何选择并生成模板。

## 一、默认必生成

无论项目类型如何，初始化器都应默认生成：

- `docs/00-初始化结果索引.md`
- `AGENTS.md`
- `.codex/COORDINATOR-SUBAGENTS.md`
- `docs/project_context.md`
- `docs/architecture/01-项目架构设计书.md`
- `docs/roadmap/01-实施路线图.md`
- `docs/workflow/current-stage-user-checklist.md`
- `docs/workflow/archive-policy.md`
- `docs/workflow/acceptance-gates.md`
- `.commonhe/session/bootstrap-handoff.md`

## 前置门禁

进入 bootstrap 前，必须满足：

1. discovery 已完成
2. proposal 已给出候选方案
3. 用户已明确拍板
4. 已形成 `user_confirmed = true` 的结构化决策结果

## 二、默认角色模板

大多数项目默认建议生成：

- `architect`
- `backend`
- `frontend`
- `reviewer`
- `qa`
- `docs`

## 三、按项目类型追加

### 若涉及数据库设计或迁移

追加：

- `database`

### 若涉及部署、CI/CD、环境管理

追加：

- `devops`

### 若涉及许可、加密、行业合规

追加：

- `compliance`

### 若涉及第三方平台集成

追加：

- `integration-generic`

## 四、按方案复杂度调节

### 快速 MVP 方案

- 角色尽量精简
- 文档保留真源与验收骨架即可

### 平衡型方案

- 默认启用 reviewer / qa / docs
- 项目上下文与路线图写得更完整

### 企业扩展型方案

- 默认补 database / devops / compliance
- 更强调回归、归档、真源治理

## 五、初始化器输出要求

初始化器在生成后，至少要告诉用户：

1. 生成了哪些文件
2. 当前阶段启用了哪些角色
3. 哪些角色或文档是按当前方案追加的
4. 哪些能力以后可以再补
5. 已晋升为长期真源的外部参考源

## 六、postcheck 强制门禁

bootstrap 落模板后，初始化器必须立即执行 `postcheck`：

1. 校验核心文件是否存在
2. 校验 `.codex/agents/*.md` 与 `docs/agents/*-handbook.md` 是否严格符合 `decision.json`
3. 若存在缺失项或冗余角色文件，则写入失败摘要并阻断“初始化成功”
4. 只有 `postcheck` 通过后，才能输出初始化成功提示

## 七、成功收口要求

`postcheck` 通过后，初始化器必须：

1. 自动把项目推进到首个实施阶段
2. 刷新 `project_context / roadmap / checklist / acceptance-gates` 为实施语义
3. 提示用户新开线程或重启 `Codex`
4. 明确当前初始化流程已结束
5. 不在当前线程继续展开业务实现、技术选型或 AI 框架设计
6. 仅提醒用户可在后续手动移除独立存在的 `CommonHE` 初始化包，不自动删除
