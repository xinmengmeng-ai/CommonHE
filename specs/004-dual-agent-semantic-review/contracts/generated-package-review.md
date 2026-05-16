# Contract: 生成物审查规范

## 1. 审查分层

生成物验收必须拆成三层，不允许把全部责任压给星梦梦。

1. `星梦梦语义审查`：判断用户需求、梦星星三方案、用户选择、角色取舍、目标软件语义和生成包说明是否一致。
2. `postcheck`：检查初始化流程是否完整落盘，目标客户端入口是否存在，状态是否收口。
3. `truth-source gate 硬门禁`：逐文件检查可确定的问题。只要硬门禁失败，即使星梦梦给出通过，也不得宣布初始化成功。

## 2. 星梦梦必须审查的语义项

星梦梦收到的 final review 上下文必须包含：

- 用户原始需求摘要与 readiness 字段。
- 梦星星完整三方案、已选方案 ID、已选方案内容。
- 目标客户端和目标客户端语义说明。
- 五项必选协作能力的选择结果。
- 生成文件清单与关键文本证据，final review 必须优先包含 `docs/workflow/current-stage-user-checklist.md`、`docs/workflow/first-sprint-contract.md`、`docs/workflow/first-task-pack.md` 的片段。
- postcheck 结果与真源规则摘要。

星梦梦必须阻断以下语义问题：

- 用户需求与已选方案或生成说明明显不一致。
- 角色选择没有理由，或者明显候选角色缺失且没有不选理由。
- `docs` 角色在 Codex 目标下必须明确以计划/交接契约形式承诺生成 `AGENTS.md` 与 `.codex/` 接管入口；pre-bootstrap 阶段不得要求这些文件已经存在。
- `reviewer` 负责语义复核、角色取舍和范围漂移，`qa` 负责可执行测试计划、关键路径验证、技术回归、跨端一致性和缺陷证据；两者职责不得互相覆盖。
- 如果省略 `AI工程师`、`安全工程师` 等自然语言角色，必须说明职责落到哪个已选角色，并补齐对应角色的执行边界，例如 backend 承接 AI API、RAG/知识库、提示词/模型参数边界，architect/backend 承接安全评估和安全实现。
- 目标客户端被错误描述成业务系统运行环境、部署平台或架构约束。
- 生成包暗示业务代码、业务实现、业务 review 或 QA 已完成。
- 生成包不是初始化协作包口径，而是业务项目成品口径。
- 生成文档出现模板腔调、半实例化模板腔调、旧业务域残留、占位符或自相矛盾说明。

星梦梦不得把业务运行依赖误判为启动器协作能力缺失。`selectedCapabilities` 只记录 `superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit` 这类工作流能力；支付、数据库、AI 平台、知识库、部署和第三方 SDK 必须在方案、角色职责或实施文档中说明，但不应要求写入 `selectedCapabilities`。

星梦梦发现业务语义问题时，必须回传梦星星修正或说明；程序不得静默替梦星星做业务角色决策。

## 3. truth-source gate 硬门禁

以下问题必须由 truth-source gate 确定性阻断，不能只依赖星梦梦判断。

### 3.1 审计闭环

- `.commonhe/session/decision.json` 必须存在。
- `.commonhe/session/meng-xingxing-output.json` 必须存在。
- `.commonhe/session/xing-mengmeng-review.json` 必须存在。
- `.commonhe/session/agent-dialogue-rounds.jsonl` 必须存在。
- `.commonhe/session/repair-decisions.json` 必须存在。
- `.commonhe/session/final-acceptance.json` 必须存在。
- `decision.json.selected_solution_id`、`meng-xingxing-output.json.selectedSolutionId`、`final-acceptance.json.selectedSolutionId` 必须一致。
- `final-acceptance.json.passed` 必须为 `true`，`reviewerAgent` 必须为 `星梦梦`，`mainAgent` 必须为 `梦星星`。

### 3.2 能力门禁

- `decision.json.selected_capabilities` 不得为空。
- `selected_capabilities` 必须包含 `superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit`。
- 五项必选能力必须记录为已选择，不能用 `required_capabilities` 代替。
- 桌面成功包必须写入 `status.capability_gate_passed=true`。
- `doctor_passed` 只是兼容字段，不能替代能力门禁。

### 3.3 目标客户端入口

- Codex 目标必须生成 `AGENTS.md`、`.codex/`、`.agents/skills/`。
- Codex 目标不得生成 `CLAUDE.md` 作为主入口。
- Claude Code 目标必须生成 `CLAUDE.md`、`.claude/`。
- Claude Code 目标不得生成 `AGENTS.md` 作为主入口。
- 目标客户端只表示后续接管软件和入口文件，不能出现在业务运行架构中。

### 3.4 接手开发包质量

- `AGENTS.md` 或 `CLAUDE.md` 的章节编号不得重复。
- 调度文档不得引用未启用角色，例如未启用 `database` 时不得出现 `backend + database`。
- `docs/workflow/current-stage-user-checklist.md` 在 `implementation_ready` 成功包中不得包含 `初始化协作包已落盘`、`先完成初始化落盘`、`再进入实施线程`、`postcheck`、`bootstrap`、`初始化落盘`、`初始化线程` 等初始化收口口径或待办。
- 当前阶段清单必须聚焦后续开发接手，例如阅读入口文档、确认首轮范围、建立任务契约、记录证据。
- `docs/workflow/first-sprint-contract.md` 必须是当前项目实例。
- `docs/workflow/first-sprint-contract.md` 不得与 `docs/workflow/sprint-contract-template.md` 完全相同。
- `docs/workflow/first-sprint-contract.md` 不得包含 `Sprint Contract 模板`、`使用前请先复制`、`任务级 Contract 脚手架`、`不代表当前项目已经存在一个已签署`、`待填写` 等模板残留。
- `docs/workflow/first-sprint-contract.md` 必须体现当前项目、第一轮、实施合同语义，并与 `docs/workflow/first-task-pack.md` 的首轮任务方向一致。
- `docs/workflow/sprint-contract-template.md` 必须保持通用模板，不得污染当前业务域词，例如具体产品名、行业名或用户需求词。

### 3.5 文本卫生和外显命名

- `.md`、`.json`、`.jsonl` 不得包含非法 ASCII 控制字符，保留 CR、LF、TAB 除外。
- 文档不得包含 Unicode replacement character 造成的乱码。
- 用户可见内容必须使用 `星星的vibecoding启动器`、`梦星星`、`星梦梦`。
- 用户可见内容不得出现 `CommonHE Init Orchestrator`、`CommonHE 正式流程` 等旧外显命名。
- 成功包不得出现 `核心实现已完成`、`QA 已完成`、`review 已完成` 之类业务实现假成功文案。

### 3.6 补救命令

- 绿色能力门禁下不得出现 `临时补救命令`。
- 只有能力缺失、探测失败或用户明确选择回退路径时，才允许生成补救说明。
- 补救说明必须写清楚当前状态和下一步，不得伪装成成功路径内容。

## 4. sanitizer 规则

`sanitizer` 只能过滤结构化证据已经证明为误报的星梦梦阻断项。

允许过滤的条件：

- 阻断项与当前 review phase 明确冲突，例如 pre-bootstrap 阶段要求最终文件证据。
- 阻断项把目标客户端误解为业务运行环境。
- 阻断项把业务应用依赖误解为启动器协作能力，例如要求把支付、数据库、AI 平台、知识库平台或第三方 SDK 写入 `selectedCapabilities`。
- 阻断项要求把启动器产品名强行写进用户业务项目名或业务架构。

禁止过滤的条件：

- truth-source gate 已经能确定失败。
- 阻断项涉及审计文件缺失、选中方案漂移、能力选择为空、入口文件错误、重复编号、未启用角色引用、陈旧初始化清单、模板污染、控制字符或旧外显命名。
- 阻断项指出业务语义遗漏，而梦星星没有给出结构化修正或拒绝理由。

如果 sanitizer 清空了星梦梦阻断项，必须在审计记录中说明是按确定性证据剔除误报，且后续仍必须执行 postcheck 和 truth-source gate。

## 5. 最终通过条件

生成物只有同时满足以下条件，才允许进入成功收口：

- 星梦梦 final review 通过。
- 梦星星与星梦梦的对话、修正和最终验收文件已落盘。
- postcheck 通过。
- truth-source gate 通过。
- status 写入 `semantic_review_passed=true`、`capability_gate_passed=true`、`truth_source_gate_passed=true`、`init_closed=true`。

任一条件不满足时，必须显示具体阻断问题，不能展示成功态。
