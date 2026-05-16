# Role Selection Rules

## 目标

定义 LLM 或初始化器如何根据项目方案决定启用哪些角色模板。

## 默认核心角色

大多数项目建议默认启用：

- `architect`
- `backend`
- `frontend`
- `reviewer`
- `qa`
- `docs`

## 条件追加

### 若项目涉及数据库设计、迁移、索引、兼容性

追加：

- `database`

### 若项目涉及部署、CI/CD、环境管理、回滚策略

追加：

- `devops`

### 若项目涉及许可、加密、法规、行业合规

追加：

- `compliance`

### 若项目需要第三方平台对接

根据实际平台追加集成角色：

- `integration-feishu`
- `integration-volcano`
- `integration-coze`
- 或其他 `integration-*`

## 轻量展示型项目

对于 `landing-page`、`solution-site`、`showcase-site` 等项目，当前阶段默认应以展示交付优先：

- 推荐当前启用：
  - `frontend`
  - `reviewer`
  - `docs`
- 复杂度较高时可补：
  - `architect`
- 默认延后：
  - `backend`
  - `database`
  - `devops`
  - `compliance`
  - `integration-*`

## 选择原则

1. 优先按业务需求选角色，不按技术炫技选角色
2. 能不启用的角色不要过早启用
3. 若用户当前只是快速 MVP，不必一开始启用完整企业化角色集
4. 若用户目标明确偏企业化，应提前启用 reviewer / qa / docs 以及必要的扩展角色
5. bootstrap 后的 AI Agents team 必须严格以已确认的 `decision.json` 为准，不得擅自扩角色
6. 不得把普通业务系统需求自动膨胀成 AI 平台、AI 框架或额外工程体系的角色集
7. 当前阶段角色与后续可启用角色必须分层，不得混写
