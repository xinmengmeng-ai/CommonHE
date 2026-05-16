# Decision Output Template

## 目标

在用户明确拍板后，把 proposal 结果收口为可供初始化器消费的结构化决策结果。

## 输出原则

- 只有用户确认后才能输出最终 decision
- decision 应描述“本次要生成什么”，而不是限制未来所有演进
- 核心字段尽量轻量，扩展字段按需补充

## 推荐输出模板

```json
{
  "user_confirmed": true,
  "project_name": "{{project_name}}",
  "project_type": "{{project_type}}",
  "solution_mode": "{{solution_mode}}",
  "enabled_roles": [
    {{enabled_roles_json}}
  ],
  "integrations": [
    {{integrations_json}}
  ]
}
```

## 字段说明

- `user_confirmed`
  - 必须为 `true`
  - 表示用户已明确拍板

- `project_name`
  - 当前项目名称

- `project_type`
  - 例如：`portal-site`、`web-app`、`saas-platform`

- `solution_mode`
  - 例如：`lean`、`fast-mvp`、`balanced`、`enterprise`

- `enabled_roles`
  - 当前初始化需要启用的角色模板

- `integrations`
  - 当前初始化需要生成的第三方集成角色

## 轻量项目示例

```json
{
  "user_confirmed": true,
  "project_name": "BrandPortal",
  "project_type": "portal-site",
  "solution_mode": "lean",
  "enabled_roles": [
    "frontend",
    "reviewer",
    "docs"
  ],
  "integrations": []
}
```

## 平衡型平台示例

```json
{
  "user_confirmed": true,
  "project_name": "OpsPlatform",
  "project_type": "saas-platform",
  "solution_mode": "balanced",
  "enabled_roles": [
    "architect",
    "backend",
    "frontend",
    "reviewer",
    "qa",
    "docs",
    "database",
    "devops"
  ],
  "integrations": [
    { "name": "feishu", "display_name": "飞书" },
    { "name": "coze", "display_name": "Coze" }
  ]
}
```
