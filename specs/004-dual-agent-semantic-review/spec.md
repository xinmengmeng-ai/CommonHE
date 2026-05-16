# Feature Spec: 双 Agent 语义验收闭环

## 目标

把桌面主流程从“梦星星单 Agent 生成 + 静态 postcheck”升级为“梦星星生成，星梦梦极端挑刺复核，双方结构化修正，通过后才允许 bootstrap/postcheck 成功收口”。

## 核心规则

- `梦星星` 是主 Agent，负责用户对话、需求理解、三方案、团队推断、角色选择理由与不选理由。
- `星梦梦` 是 CommonHE 语义验收 Agent，负责检查用户需求、梦星星输出、用户选择、目标软件、能力状态、生成文件和真源规则。
- 双 Agent 编排必须供应商无关，不得把核心运行时绑定到 OpenAI Agents SDK、LangGraph、AutoGen、Anthropic SDK 或其他单一供应商框架。
- 星梦梦发现疑似遗漏、矛盾或语义跑偏时，必须回传给梦星星修正或说明，不得由程序静默替代业务判断。
- 静态 truth-source gate 和 postcheck 是硬底线；星梦梦是语义门禁，两者都不能被跳过。

## 成功标准

- 梦星星三方案必须包含 `roleRationale` 与 `omittedRoleRationale`。
- `.commonhe/session/` 必须写入 `meng-xingxing-output.json`、`xing-mengmeng-review.json`、`agent-dialogue-rounds.jsonl`、`repair-decisions.json`、`final-acceptance.json`。
- `final-acceptance.json.passed=true` 前，桌面主流程不得执行 bootstrap 成功收口。
- 语义验收失败必须显示星梦梦阻断项，不得跳成功页面。
- `decision.json.selected_solution_id` 必须与 `meng-xingxing-output.json.selectedSolutionId`、`final-acceptance.json.selectedSolutionId` 一致。
- `decision.json.selected_capabilities` 必须保留五项必选能力的 selected=true 记录；成功状态必须写入 `status.capability_gate_passed=true`。
- 成功协作包不得包含绿色路径下的临时补救块、非法 ASCII 控制字符、旧外显产品名或暗示业务实现/review/QA 已完成的文案。
- 成功协作包必须能被 Codex 直接接手开发：`AGENTS.md` 编号不重复；调度说明只引用真实启用角色；当前阶段清单聚焦实施接手而不是初始化收口；首轮 Sprint Contract 与通用模板必须分离，并且不得残留“模板/复制/待填写”等半实例化模板腔。

## 生成物审查规范

- 生成物验收必须遵守 `contracts/generated-package-review.md`。
- 星梦梦负责语义审查，truth-source gate 负责确定性硬门禁；二者缺一不可。
- sanitizer 只能剔除已被结构化证据证明的星梦梦误报，不得吞掉 truth-source gate 可确定的问题。
