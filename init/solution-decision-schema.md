# Solution Decision Schema

## 目标

定义 LLM 在 proposal 结束后，应产出的结构化决策结果格式。

该结果用于驱动初始化器决定：

- 当前阶段应该启用哪些角色模板
- 哪些角色与能力延后到后续阶段
- 是否需要生成集成角色模板
- 哪些外部参考源需要晋升为长期真源
- bootstrap 后应推进到哪个实施阶段

## 第一版建议字段

```json
{
  "user_confirmed": true,
  "project_type": "solution-site",
  "delivery_mode": "solution-site",
  "solution_mode": "fast-mvp",
  "enabled_roles": ["frontend", "reviewer", "docs"],
  "recommended_roles_now": ["frontend", "reviewer", "docs"],
  "available_roles_later": ["backend", "database", "devops"],
  "integrations": [
    {
      "name": "feishu",
      "display_name": "飞书"
    }
  ],
  "external_references": [
    {
      "type": "style-reference",
      "path": "E:\\WorkSoft\\bytelink-ui-model",
      "purpose": "视觉风格与界面表达参考",
      "must_read": true
    }
  ],
  "current_stage": "implementation-v1",
  "current_stage_goal": "完成当前展示型项目的首个可交付版本",
  "primary_workstream": "showcase-site"
}
```

## 字段说明

### `user_confirmed`

- 类型：布尔值
- 含义：用户是否已经明确拍板当前方案
- 规则：未显式为 `true` 时，不得进入自动 bootstrap

### `enabled_roles`

- 类型：数组
- 含义：本项目当前应启用的角色模板清单

### `delivery_mode`

- 类型：字符串
- 含义：当前阶段的真实交付形态
- 建议值：
  - `landing-page`
  - `solution-site`
  - `showcase-site`
  - `web-app`
  - `saas-platform`
  - `internal-tool`

### `recommended_roles_now`

- 类型：数组
- 含义：当前阶段立即启用的角色

### `available_roles_later`

- 类型：数组
- 含义：后续阶段可启用但本轮默认不激活的角色

### `integrations`

- 类型：数组
- 含义：需要生成的第三方集成角色

每项建议包含：

- `name`
- `display_name`

### `external_references`

- 类型：数组
- 含义：需要晋升为长期真源的外部参考源
- 推荐字段：
  - `type`
  - `path`
  - `purpose`
  - `must_read`

### `current_stage`

- 类型：字符串
- 含义：bootstrap 后推进到的实施阶段

### `current_stage_goal`

- 类型：字符串
- 含义：当前阶段真正要交付的目标

## 后续可扩展字段

- `project_type`
- `solution_mode`
- `architecture_style`
- `doc_depth`
- `truth_source_profile`
- `risk_profile`
- `stage_constraints`
- `deferred_capabilities`
- `implementation_checklist_seed`
- `implementation_acceptance_seed`

## 强制门禁

- `decision.json` 不是 proposal 的中间草稿
- 它应表示“用户已确认后的结构化结果”
- 若 `user_confirmed` 不是 `true`，初始化器不得自动生成初始化协作包
