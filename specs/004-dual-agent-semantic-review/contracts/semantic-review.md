# Contract: 双 Agent 语义验收

## 成功前置

`final-acceptance.json.passed` 必须为 `true`，并且 postcheck/truth-source gate 必须通过。

`decision.json.selected_solution_id`、`meng-xingxing-output.json.selectedSolutionId`、`final-acceptance.json.selectedSolutionId` 必须指向同一个用户选择。

`decision.json.selected_capabilities` 必须包含五项必选能力且均为 `selected=true`；桌面成功包必须记录 `status.capability_gate_passed=true`。

## 失败行为

如果星梦梦输出 blocking issues：

- 未达到最大修复轮次时，必须回传梦星星修正或说明，并重新提交星梦梦审查。
- 只有达到最大修复轮次仍未通过时，才允许阻断成功收口。
- 阻断时不执行目标工作区 bootstrap 成功收口。
- UI 展示星梦梦阻断项与已执行修复轮次。
- session 写入审计文件。

星梦梦不能把 `selectedCapabilities` 当作业务运行依赖清单。`selectedCapabilities` 只记录启动器/协作包工作流能力：`superpowers`、`agent-browser`、`chrome-devtools`、`GitNexus`、`Speckit`。支付、数据库、AI 平台、知识库、部署和第三方 SDK 属于业务方案/实施边界，不因未出现在 `selectedCapabilities` 中阻断。

如果生成包出现空 selected 能力、选中方案漂移、绿色路径临时补救块、非法控制字符、旧外显产品名或业务实现假成功文案，truth-source gate 必须阻断。

## 生成物审查合同

最终生成包必须同时遵守 [generated-package-review.md](./generated-package-review.md)。

该合同明确区分 `星梦梦语义审查`、postcheck 与 `truth-source gate 硬门禁` 的职责边界。重复编号、未启用角色引用、陈旧初始化清单、Sprint Contract 实例/模板混用、控制字符、旧外显命名、绿色路径临时补救命令等问题必须由确定性门禁阻断，不能只依赖星梦梦主观判断。
