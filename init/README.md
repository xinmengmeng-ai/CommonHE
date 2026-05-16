# CommonHE Init

## 目标

`init/` 定义通用初始化流程。

这个流程必须把 LLM 作为 AI 产品经理来使用，而不是把用户当成技术架构师来盘问。

## 默认流程

1. `Doctor`
   - 先诊断五项硬门禁
   - 同时检查入口 / 解压层级是否正确
2. `Precheck`
   - 在 `doctor` 通过后继续做能力校验落盘
3. `Autodiscovery`
   - 对老项目默认零提问自动分析仓库结构与现有上下文
4. `Bootstrap`
   - 直接生成项目 HE 骨架、required capability 文档、高级 workflow 文档与 kickoff docs
5. `Postcheck`
   - 强制校验 AI Agents team、required capability 文档、agent 能力声明、kickoff docs 与 probe 记录
6. `Closure`
   - 通过后收口初始化线程，要求新开线程或重启 `Codex`

## 兼容流程

仅当目录基本为空、是模板目录，或没有明显项目信号时，保留：

1. `Doctor`
2. `Precheck`
3. `Discovery`
4. `Proposal`
5. `Confirm`
6. `Bootstrap`
7. `Postcheck`

## 基本要求

- 先问业务目标，不先问技术栈
- 用业务语言解释技术差异
- 老项目默认不追问用户，由系统直接分析并生成保守决策
- 五项能力未通过前，不得进入 bootstrap
- `doctor` 未通过前，不得进入 `precheck`
- `postcheck` 未通过前，不得宣布初始化成功
- `docs/workflow/current-stage-user-checklist.md` 必须保持为实施态专用文档，不得混入 init-only 内容
- `evaluator-protocol`、`grading-criteria`、`sprint-contract-template`、kickoff docs 必须作为项目内长期文档生成
- 初始化线程只负责生成 HE 协作工程，不继续展开业务实现或 AI 框架设计
